# column-rs

Rust 实现的 `column(1)` 基础版本，支持 Linux / macOS / Windows。

## 功能

- 从标准输入或文件读取文本
- 默认按列表分栏输出（接近 `column` 默认模式）
- 支持 `-t` 表格模式并按列对齐输出（接近 `column -t` 基础行为）
- 支持基础参数：`-h/-V/-t/-c/-x/-L/-s/-o/-J/-N/-n/-d/-H`

## 兼容性进度

与 `column(1)` 的差异与收敛计划见：

- `docs/column1-compatibility.md`

## 使用

```bash
cargo run --
printf "name age\nalice 8\nbob 12\n" | cargo run --
```
