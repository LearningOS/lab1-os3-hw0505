//! Types related to task management

use super::TaskContext;
use crate::config::{MAX_SYSCALL_NUM};
//use crate::syscall::{TaskInfo};

#[derive(Copy, Clone)]
/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub task_run_last_time: usize,
    pub task_time: usize,
    //pub task_info:TaskInfo,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    // LAB1: Add whatever you need about the Task.
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Copy, Clone)]
pub struct TaskInfo {
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time: usize,
}

// impl TaskInfo {
//     pub fn init() -> Self {
//         TaskInfo{
//             status: TaskStatus::UnInit,
//             syscall_times: [0; MAX_SYSCALL_NUM],
//             time: 0,
//         }
//     }
// }