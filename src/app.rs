#![forbid(unsafe_code)]

use crate::libloader::LibLoader;
use crate::libloader::DYLIB_EXT;
use crate::prelude::*;
use crate::style::*;
use crate::window::*;

use std::{collections::HashMap, env, error, fmt::Debug, mem, path::Path, time::Duration, time::Instant};

use glutin::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowId,
};

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum On {
    MouseDown,
    MouseUp,
    Hover,

    Change, // Can be used by widgets to signal that they have changed
    Focus,
    Blur, // TODO - cache id on focus, so blur doesn't have to search
}

#[derive(Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Stage {
    Idle,
    Paint,
    Layout,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTask {
    Yes,
    No,
}

pub type EventCallback<T> = dyn Fn(&mut T, &mut App<T>) -> Stage;
pub type StyleCallback<T> = dyn Fn(&T, &mut Style);
pub type TaskCallback<T> = dyn Fn(&mut T, &mut App<T>) -> (Stage, StopTask);
pub type AnimCallback<T> = dyn Fn(&mut T, Duration) -> (Stage, StopTask);
pub type ViewCallback<T> = for<'a> fn(&T, &'a Alloc) -> Node<'a, T>;

struct Task<T> {
    window_id: Option<WindowId>,
    last_run: Instant,
    frequency: Duration,
    callback: Box<TaskCallback<T>>,
}

pub struct App<T> {
    event_loop: Option<EventLoop<()>>,
    loader: Option<LibLoader>,
    new_windows: Vec<WindowDesc<T>>,
    windows: HashMap<WindowId, RosinWindow<T>>,
    current_window: Option<WindowId>,
    stylesheet: Stylesheet,
    tasks: Vec<Task<T>>,
}

impl<T: 'static> Default for App<T> {
    fn default() -> Self {
        Self::new()
    }
}

// TODO add event_filters and event_handlers?
// Need some way to access raw events for pen pressure, etc
impl<T: 'static> App<T> {
    pub fn new() -> Self {
        Self {
            event_loop: Some(EventLoop::new()),
            loader: None,
            new_windows: Vec::new(),
            windows: HashMap::new(),
            current_window: None,
            stylesheet: Stylesheet::default(),
            tasks: Vec::new(),
        }
    }

    pub fn add_window(mut self, desc: WindowDesc<T>) -> Self {
        self.new_windows.push(desc);
        self
    }

    pub fn use_stylesheet(mut self, stylesheet: Stylesheet) -> Self {
        self.stylesheet = stylesheet;
        self
    }

    // TODO add_anim_task

    // Similar to setInterval in JS
    pub fn add_task(&mut self, window_id: Option<WindowId>, frequency: Duration, callback: Box<TaskCallback<T>>) {
        self.tasks.push(Task {
            window_id,
            last_run: Instant::now(),
            frequency: Duration::from_millis(10).max(frequency),
            callback,
        });
    }

    pub fn current_window(&self) -> Option<WindowId> {
        self.current_window
    }

    // TODO - trigger a change event on self, and every ancestor node (need self for when a widget has only one node)
    pub fn emit_change(&mut self) {
        // make sure to stop infinite loops of change handlers emitting changes
        // probably only one event per frame, so no need to batch them up
        todo!();
    }

    pub fn focus_on(&mut self, key: Key) {
        todo!();
    }

    // Avoids linear searching through all nodes
    pub fn focus_on_ancestor(&mut self, key: Key) {
        todo!();
    }

    pub fn blur(&mut self) {
        todo!();
    }

    pub fn run(mut self, mut state: T) -> Result<(), Box<dyn error::Error>> {
        let event_loop = self.event_loop.take().ok_or("[Rosin] Already launched")?;
        if self.new_windows.is_empty() {
            return Err("[Rosin] No windows".into());
        }

        if cfg!(debug_assertions) && cfg!(feature = "hot-reload") {
            // Use the name of the current binary to find the library
            let lib_path = env::current_dir()?.join(Path::new(&env::args().next().unwrap()).with_extension(DYLIB_EXT));
            self.loader = Some(LibLoader::new(lib_path).expect("[Rosin] Hot-reload: Failed to init"));
        }

        if cfg!(debug_assertions) {
            self.add_task(
                None,
                Duration::from_millis(100),
                Box::new(|_, app| {
                    let mut stage = match app.stylesheet.poll() {
                        Ok(true) => Stage::Build,
                        Ok(false) => Stage::Idle,
                        Err(error) => {
                            eprintln!(
                                "[Rosin] Failed to reload stylesheet: {:?} Error: {:?}",
                                app.stylesheet.path, error
                            );
                            Stage::Idle
                        }
                    };

                    if cfg!(feature = "hot-reload") {
                        if let Some(loader) = &mut app.loader {
                            match loader.poll() {
                                Ok(true) => stage = Stage::Build,
                                Err(error) => {
                                    eprintln!("[Rosin] Failed to hot-reload. Error: {:?}", error);
                                }
                                _ => (),
                            }
                        }
                    }

                    (stage, StopTask::No)
                }),
            );
        }

        let mut active_tasks = Vec::new();
        let mut stopped_task_ids = Vec::new();

        //TODO what to do about unwraps in the event loop? Can't return error...
        event_loop.run(move |event, event_loop, control_flow| {
            // Run tasks
            // TODO - find a better place to run them. In response to which sytem events?
            if self.tasks.is_empty() {
                *control_flow = ControlFlow::Wait;
            } else {
                mem::swap(&mut self.tasks, &mut active_tasks);

                let mut next_update = Instant::now() + Duration::from_secs(3600);
                let mut new_stage = Stage::Idle;

                for (i, task) in active_tasks.iter_mut().enumerate() {
                    if Instant::now().duration_since(task.last_run) >= task.frequency {
                        task.last_run = Instant::now();
                        let (stage, stoptask) = (task.callback)(&mut state, &mut self);
                        if let Some(window_id) = task.window_id {
                            self.windows.get_mut(&window_id).unwrap().update_stage(stage);
                        } else {
                            new_stage = new_stage.max(stage);
                        }

                        if stoptask == StopTask::Yes {
                            stopped_task_ids.push(i);
                            continue;
                        }
                    }
                    next_update = next_update.min(task.last_run + task.frequency);
                }

                stopped_task_ids.sort_unstable();
                for id in stopped_task_ids.drain(..).rev() {
                    active_tasks.swap_remove(id);
                }

                self.tasks.append(&mut active_tasks);

                for (_, window) in self.windows.iter_mut() {
                    window.update_stage(new_stage);
                }

                *control_flow = ControlFlow::WaitUntil(next_update);
            }

            // Handle Events
            match event {
                Event::WindowEvent { event, window_id } => {
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            self.windows.get_mut(&window_id).unwrap().resize(physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            self.windows.get_mut(&window_id).unwrap().resize(*new_inner_size);
                        }
                        WindowEvent::CloseRequested => {
                            // Remove any tasks associated with the closing window
                            self.tasks.retain(|task| {
                                if let Some(id) = task.window_id {
                                    id != window_id
                                } else {
                                    true
                                }
                            });

                            // TODO - Remove anim tasks

                            // Drops the window, causing it to close.
                            self.windows.remove(&window_id);

                            if self.windows.is_empty() {
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(window_id) => {
                    self.windows
                        .get_mut(&window_id)
                        .unwrap()
                        .redraw(&state, &self.stylesheet, &self.loader)
                        .unwrap();
                }
                _ => {}
            }

            // Build new windows
            for desc in self.new_windows.drain(..) {
                let window = RosinWindow::new(desc, event_loop).unwrap();
                self.windows.insert(window.id(), window);
            }
        });
    }
}
