/*
 * @Date: 2023-09-06 15:44:25
 * @LastEditors: WWW
 * @LastEditTime: 2023-09-06 15:45:02
 * @FilePath: \mitm\src\utilts.rs
 */
pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}