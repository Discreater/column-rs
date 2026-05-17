# column-rs

Rust 实现的 `column(1)` 基础版本，支持 Linux / macOS / Windows。

## 功能

- 从标准输入或文件读取文本
- 以空白字符分列并按列对齐输出（等价于 `column -t` 的基础行为）
- 默认输出对齐表格

## 使用

```bash
cargo run --
printf "name age\nalice 8\nbob 12\n" | cargo run --
```
