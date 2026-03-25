// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_events::{Event, resource};
use serde::Serialize;
use uuid::Uuid;

use crate::EventSender;

#[derive(Clone)]
pub struct MemoryResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
}

impl<T> MemoryResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: EventSender<T>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::memory::Init) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Init(init)).into(),
        ))
    }

    pub fn operating(&self, id: Uuid, operating: resource::memory::Operating) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Operating(operating))
                .into(),
        ))
    }

    pub fn resizing(&self, id: Uuid, resizing: resource::memory::Resizing) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Resizing(resizing))
                .into(),
        ))
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::memory::Finalizing) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Finalizing(finalizing))
                .into(),
        ))
    }

    pub fn exit(&self, id: Uuid, exit: resource::memory::Exit) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Memory(resource::memory::MemoryEvent::Exit(exit)).into(),
        ))
    }
}

#[derive(Clone)]
pub struct ProcessorResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
}

impl<T> ProcessorResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: EventSender<T>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::processor::Init) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Init(init))
                .into(),
        ))
    }

    pub fn operating(&self, id: Uuid, operating: resource::processor::Operating) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Operating(
                operating,
            ))
            .into(),
        ))
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::processor::Finalizing) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Finalizing(
                finalizing,
            ))
            .into(),
        ))
    }

    pub fn exit(&self, id: Uuid, exit: resource::processor::Exit) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Processor(resource::processor::ProcessorEvent::Exit(exit))
                .into(),
        ))
    }
}

#[derive(Clone)]
pub struct ChannelResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
}

impl<T> ChannelResourceObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: EventSender<T>) -> Self {
        Self { tx }
    }

    pub fn init(&self, id: Uuid, init: resource::channel::Init) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Init(init)).into(),
        ))
    }

    pub fn operating(&self, id: Uuid, operating: resource::channel::Operating) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Operating(operating))
                .into(),
        ))
    }

    pub fn finalizing(&self, id: Uuid, finalizing: resource::channel::Finalizing) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Finalizing(
                finalizing,
            ))
            .into(),
        ))
    }

    pub fn exit(&self, id: Uuid, exit: resource::channel::Exit) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Channel(resource::channel::ChannelEvent::Exit(exit)).into(),
        ))
    }
}

#[derive(Clone)]
pub struct ResourceGroupObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    tx: EventSender<T>,
}

impl<T> ResourceGroupObserver<T>
where
    T: From<resource::ResourceEvent> + Serialize + Send + std::fmt::Debug + 'static,
{
    pub fn new(tx: EventSender<T>) -> Self {
        Self { tx }
    }

    pub fn group(&self, id: Uuid, group: resource::GroupEvent) {
        self.tx.send(Event::new_now(
            id,
            resource::ResourceEvent::Group(group).into(),
        ))
    }
}
