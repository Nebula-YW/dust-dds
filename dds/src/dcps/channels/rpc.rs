use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex, signal::Signal,
};

use crate::{
    dcps::dcps_mail::{DcpsMail, DcpsReply},
    infrastructure::error::{DdsError, DdsResult},
    runtime::{Either, select_future},
};

/// Mailbox owned by the participant factory actor loop to receive requests and send responses.
pub struct RpcMailbox {
    closed: AtomicBool,
    close_signal: Signal<CriticalSectionRawMutex, ()>,
    rpc_lock: Mutex<CriticalSectionRawMutex, ()>,
    request_channel: Channel<CriticalSectionRawMutex, DcpsMail, 1>,
    response_channel: Channel<CriticalSectionRawMutex, DcpsReply, 1>,
}

impl RpcMailbox {
    /// Creates a new RPC mailbox.
    pub fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
            close_signal: Signal::new(),
            rpc_lock: Mutex::new(()),
            request_channel: Channel::new(),
            response_channel: Channel::new(),
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.close_signal.signal(());
    }

    pub fn clear_pending(&self) {
        self.request_channel.clear();
        self.response_channel.clear();
    }

    /// Receives a request from a caller.
    pub async fn receive_request(&self) -> DcpsMail {
        self.request_channel.receive().await
    }

    /// Sends a response back to the waiting caller.
    pub async fn send_reply(&self, reply: DcpsReply) {
        self.response_channel.send(reply).await;
    }
}

impl Default for RpcMailbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Client handle held by async entities to perform request-response RPC calls with the backend.
#[derive(Clone)]
pub struct RpcClient {
    mailbox: Arc<RpcMailbox>,
}

impl RpcClient {
    /// Creates a new RPC client wrapping a shared mailbox.
    pub fn new(mailbox: Arc<RpcMailbox>) -> Self {
        Self { mailbox }
    }

    pub fn close(&self) {
        self.mailbox.close();
    }
    pub fn clear_pending(&self) {
        self.mailbox.clear_pending();
    }

    /// Sends an RPC request to the backend actor and awaits its response.
    pub async fn call(&self, mail: DcpsMail) -> DdsResult<DcpsReply> {
        if self.mailbox.closed.load(Ordering::Acquire) {
            return Err(DdsError::AlreadyDeleted);
        }
        let _guard = self.mailbox.rpc_lock.lock().await;
        if self.mailbox.closed.load(Ordering::Acquire) {
            return Err(DdsError::AlreadyDeleted);
        }
        // Only the lock owner waits on this signal; queued callers check closed after acquiring it.
        match select_future(self.mailbox.close_signal.wait(), async {
            let _ = self.mailbox.response_channel.try_receive();
            self.mailbox.request_channel.send(mail).await;
            self.mailbox.response_channel.receive().await
        })
        .await
        {
            Either::A(()) => Err(DdsError::AlreadyDeleted),
            Either::B(reply) => Ok(reply),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod close_tests {
    use super::*;
    use crate::dcps::dcps_mail::ParticipantFactoryMail;
    use std::time::Duration;
    fn mail() -> DcpsMail {
        DcpsMail::ParticipantFactory(ParticipantFactoryMail::GetQos)
    }

    #[tokio::test]
    async fn closing_wakes_active_rpc_and_rejects_queued_callers() {
        let mailbox = Arc::new(RpcMailbox::new());
        let client = RpcClient::new(mailbox.clone());
        let mut tasks = Vec::new();
        for _ in 0..16 {
            let client = client.clone();
            tasks.push(tokio::spawn(async move { client.call(mail()).await }));
        }
        tokio::time::timeout(Duration::from_secs(1), mailbox.receive_request())
            .await
            .unwrap();
        mailbox.close();
        for task in tasks {
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(1), task)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(DdsError::AlreadyDeleted)
            ));
        }
        assert!(matches!(
            client.call(mail()).await,
            Err(DdsError::AlreadyDeleted)
        ));
    }

    #[tokio::test]
    async fn closing_wakes_rpc_blocked_sending_request() {
        let mailbox = Arc::new(RpcMailbox::new());
        assert!(mailbox.request_channel.try_send(mail()).is_ok());
        let client = RpcClient::new(mailbox.clone());
        let task = tokio::spawn(async move { client.call(mail()).await });
        tokio::task::yield_now().await;
        mailbox.close();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap(),
            Err(DdsError::AlreadyDeleted)
        ));
        mailbox.clear_pending();
        assert!(mailbox.request_channel.is_empty());
    }
}
