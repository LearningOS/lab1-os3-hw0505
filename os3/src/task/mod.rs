//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see [`__switch`]. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
//pub use crate::syscall::process::{TaskInfo};

use crate::timer::get_time_us;

use lazy_static::*;
pub use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus, TaskInfo};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        //println!("    TASK_MANAGER  instance1");
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_run_last_time: 0,
            task_time: 0,
            //task_info:TaskInfo::init(),
            syscall_times: [0;MAX_SYSCALL_NUM],
        }; MAX_APP_NUM];
        //println!("    TASK_MANAGER  instance2");
        for (i, t) in tasks.iter_mut().enumerate().take(num_app) {
            t.task_cx = TaskContext::goto_restore(init_app_cx(i));
            t.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        //add by hw
        task0.task_run_last_time = get_time_us();

        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        //add by hw
        //let last_time = inner.tasks[current].task_run_last_time;
        //inner.tasks[current].task_time += (get_time_us() - inner.tasks[current].task_run_last_time)/1000;
        inner.tasks[current].task_status = TaskStatus::Ready;
        
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        //add by hw
        //inner.tasks[current].task_time += (get_time_us() - inner.tasks[current].task_run_last_time)/1000;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            //add by hw
            if inner.tasks[next].task_run_last_time == 0 {
                inner.tasks[next].task_run_last_time = get_time_us();
            }
            


            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
    fn add_syscall_num(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        //let syscall_id:u32 = (*syscall_id) as u32;
        inner.tasks[current].syscall_times[syscall_id] += 1;
        //println!("    task_id: {} syscall_num: {}",current, inner.tasks[current].syscall_num);
    }
    fn get_run_time(&self) -> usize{
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let now = get_time_us();
        inner.tasks[current].task_time = (now - inner.tasks[current].task_run_last_time)/1000 ;
        inner.tasks[current].task_time
    }
    fn get_status(&self) -> TaskStatus{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status
    }
    fn get_syscall(&self) -> [u32;MAX_SYSCALL_NUM]{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times
    }

    fn get_current_taskinfo(&self, ti: *mut TaskInfo) -> isize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let now = get_time_us();
        // unsafe {
        //     (*ti).status = inner.tasks[current].task_status;
        //     (*ti).syscall_times = inner.tasks[current].task_info.syscall_times;
        //     (*ti).time = (now - inner.tasks[current].task_run_last_time)/1000 ;
        // }
        0
    }
    fn print(&self) {
        let mut inner = self.inner.exclusive_access();
        println!("    current_task: {}",inner.current_task);
    }

    // LAB1: Try to implement your function to update or get task info!
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
    //let time = TASK_MANAGER.get_run_time();
    //println!("    run_time: {}", time);
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

// LAB1: Public functions implemented here provide interfaces.
// You may use TASK_MANAGER member functions to handle requests.
pub fn add_syscall_num(syscall_id: usize) {
    TASK_MANAGER.add_syscall_num(syscall_id);
}
pub fn get_run_time() -> usize{
    TASK_MANAGER.get_run_time()
}
pub fn get_status() -> TaskStatus {
    TASK_MANAGER.get_status()
}
pub fn get_syscall() -> [u32;MAX_SYSCALL_NUM] {
    TASK_MANAGER.get_syscall()
}
pub fn get_task_info(ti: *mut TaskInfo) -> isize {
    TASK_MANAGER.get_current_taskinfo(ti)
}