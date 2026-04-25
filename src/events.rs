use std::time::Duration;

use crossterm::event::{Event as CtEvent, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::api::{Source, Summary};

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize,
    Loaded(Box<Summary>),
    LoadFailed(String),
}

pub struct EventLoop {
    pub rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
    source: Source,
}

impl EventLoop {
    pub fn new(source: Source, refresh: Duration, render_tick: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let me = Self { rx, tx: tx.clone(), source };
        spawn_input(tx.clone());
        spawn_render_tick(tx.clone(), render_tick);
        spawn_refresh(tx, me.source.clone(), refresh);
        me
    }

    pub fn refresh_now(&self) {
        let tx = self.tx.clone();
        let source = self.source.clone();
        tokio::spawn(async move {
            send_load(&source, &tx).await;
        });
    }
}

fn spawn_input(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut events = EventStream::new();
        while let Some(ev) = events.next().await {
            match ev {
                Ok(CtEvent::Key(k)) if k.kind == KeyEventKind::Press => {
                    if tx.send(AppEvent::Key(k)).is_err() {
                        break;
                    }
                }
                Ok(CtEvent::Resize(_, _)) => {
                    let _ = tx.send(AppEvent::Resize);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

fn spawn_render_tick(tx: mpsc::UnboundedSender<AppEvent>, every: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(every);
        loop {
            interval.tick().await;
            if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
}

fn spawn_refresh(tx: mpsc::UnboundedSender<AppEvent>, source: Source, every: Duration) {
    tokio::spawn(async move {
        send_load(&source, &tx).await;
        let mut interval = tokio::time::interval(every);
        interval.tick().await;
        loop {
            interval.tick().await;
            send_load(&source, &tx).await;
        }
    });
}

async fn send_load(source: &Source, tx: &mpsc::UnboundedSender<AppEvent>) {
    let evt = match source.fetch_summary().await {
        Ok(s) => AppEvent::Loaded(Box::new(s)),
        Err(e) => AppEvent::LoadFailed(format!("{e:#}")),
    };
    let _ = tx.send(evt);
}
