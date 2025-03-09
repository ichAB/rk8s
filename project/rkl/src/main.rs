mod task; // 声明 task 模块

use task::task::TaskRunner; // 导入 TaskRunner
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 加载并解析 pod.yaml
    let task_runner = TaskRunner::from_file("./pod.yaml")?;
    
    // 打印解析结果
    println!("Parsed PodTask: {:?}", task_runner.task);

    Ok(())
}