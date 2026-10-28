use super::*;
use crate::{
    builtin_topics::BuiltInTopicKey,
    dcps::channels::oneshot::{OneshotReceiver, oneshot},
    infrastructure::qos_policy::DEFAULT_RELIABILITY_QOS_POLICY_DATA_READER_AND_TOPICS,
    transport::types::{DurabilityKind, ENTITYID_UNKNOWN, Guid, ReaderProxy, ReliabilityKind},
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

fn writer() -> UserDefinedDataWriter {
    let mut writer = UserDefinedDataWriter::new(
        InstanceHandle::new([0; 16]),
        RtpsStatefulWriter::new(Guid::from([0; 16]), 1024),
        Arc::from("AcknowledgmentRemoval"),
        None,
        StatusMask::default(),
        DataWriterQos::default(),
        KeyHolderType::default(),
    );
    writer.last_change_sequence_number = 1;
    writer
}

fn add_reader(writer: &mut UserDefinedDataWriter, id: u8) -> InstanceHandle {
    let key = [id; 16];
    writer.transport_writer.add_matched_reader(ReaderProxy {
        remote_reader_guid: key.into(),
        remote_group_entity_id: ENTITYID_UNKNOWN,
        reliability_kind: ReliabilityKind::Reliable,
        durability_kind: DurabilityKind::Volatile,
        unicast_locator_list: Vec::new(),
        multicast_locator_list: Vec::new(),
        expects_inline_qos: false,
    });
    writer
        .matched_subscription_list
        .push(SubscriptionBuiltinTopicData {
            key: BuiltInTopicKey { value: key },
            participant_key: Default::default(),
            topic_name: Default::default(),
            type_name: Default::default(),
            type_information: None,
            durability: Default::default(),
            deadline: Default::default(),
            latency_budget: Default::default(),
            liveliness: Default::default(),
            reliability: DEFAULT_RELIABILITY_QOS_POLICY_DATA_READER_AND_TOPICS,
            ownership: Default::default(),
            destination_order: Default::default(),
            user_data: Default::default(),
            time_based_filter: Default::default(),
            presentation: Default::default(),
            partition: Default::default(),
            topic_data: Default::default(),
            group_data: Default::default(),
            representation: Default::default(),
            type_consistency: Default::default(),
        });
    writer.publication_matched_status.current_count += 1;
    InstanceHandle::new(key)
}

fn poll<T>(
    receiver: &mut OneshotReceiver<T>,
) -> Poll<Result<T, crate::infrastructure::error::DdsError>> {
    Pin::new(receiver).poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn removed_subscription_no_longer_blocks_future_acknowledgments() {
    let mut writer = writer();
    let reader = add_reader(&mut writer, 1);
    assert!(!writer.transport_writer.is_change_acknowledged(1));
    writer.remove_matched_subscription(&reader);
    assert!(writer.transport_writer.is_change_acknowledged(1));
    assert!(writer.matched_subscription_list.is_empty());
    assert_eq!(writer.publication_matched_status.current_count, 0);
    assert_eq!(writer.publication_matched_status.current_count_change, -1);
}

#[test]
fn removing_last_reader_completes_all_pending_acknowledgments() {
    let mut writer = writer();
    let reader = add_reader(&mut writer, 1);
    let (first, mut first_receiver) = oneshot();
    let (second, mut second_receiver) = oneshot();
    writer
        .wait_for_acknowledgments_notification
        .extend([first, second]);
    assert!(poll(&mut first_receiver).is_pending());
    assert!(poll(&mut second_receiver).is_pending());
    writer.remove_matched_subscription(&reader);
    assert!(matches!(poll(&mut first_receiver), Poll::Ready(Ok(Ok(())))));
    assert!(matches!(
        poll(&mut second_receiver),
        Poll::Ready(Ok(Ok(())))
    ));
    assert!(writer.wait_for_acknowledgments_notification.is_empty());
}

#[test]
fn live_unacknowledged_reader_keeps_acknowledgments_pending() {
    let mut writer = writer();
    let removed_reader = add_reader(&mut writer, 1);
    let live_reader = add_reader(&mut writer, 2);
    let (sender, mut receiver) = oneshot();
    writer.wait_for_acknowledgments_notification.push(sender);
    writer.remove_matched_subscription(&removed_reader);
    assert!(!writer.transport_writer.is_change_acknowledged(1));
    assert!(poll(&mut receiver).is_pending());
    assert_eq!(writer.publication_matched_status.current_count, 1);
    writer.remove_matched_subscription(&live_reader);
    assert!(matches!(poll(&mut receiver), Poll::Ready(Ok(Ok(())))));
}

#[test]
fn unknown_and_duplicate_removals_do_not_change_match_counts() {
    let mut writer = writer();
    let reader = add_reader(&mut writer, 1);
    writer.remove_matched_subscription(&InstanceHandle::new([9; 16]));
    assert_eq!(writer.publication_matched_status.current_count, 1);
    assert_eq!(writer.publication_matched_status.current_count_change, 0);
    assert!(!writer.transport_writer.is_change_acknowledged(1));
    writer.remove_matched_subscription(&reader);
    writer.remove_matched_subscription(&reader);
    assert_eq!(writer.publication_matched_status.current_count, 0);
    assert_eq!(writer.publication_matched_status.current_count_change, -1);
}
