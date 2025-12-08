# 🔥🔥🔥 伏羲量化 🔥🔥🔥

> 基于 **Rhai** 脚本语言的量化交易运行时，提供交易 API、时间处理和数据分析能力。

---

## 🗺️ 目录

- [Rhai 语言基础](#rhai-语言基础)
  - [基本语法](#基本语法)
  - [数据类型](#数据类型)
  - [运算符](#运算符)
  - [控制流](#控制流)
  - [数组操作](#数组操作)
  - [对象 (Map)](#对象-map)
- [Decimal 高精度计算](#decimal-高精度计算)
- [时间处理](#时间处理)
  - [Time 类型](#time-类型)
  - [Duration 类型](#duration-类型)
- [交易 API](#交易-api)
  - [常量定义](#常量定义)
  - [K 线与信号](#k-线与信号)
  - [下单函数](#下单函数)
  - [资金查询](#资金查询)
  - [持仓管理](#持仓管理)
  - [订单管理](#订单管理)
  - [合约信息](#合约信息)
- [数据处理 (Polars)](#数据处理-polars)
  - [常量](#polars-常量)
  - [Series](#series)
  - [DataFrame](#dataframe)
  - [LazyFrame](#lazyframe)
  - [Expr 表达式](#expr-表达式)
  - [全局函数](#全局函数)
- [策略回调函数](#策略回调函数)
- [完整示例](#完整示例)
- [注意事项](#注意事项)

---

## 🚀 Rhai 语言基础

Rhai 是一门轻量级嵌入式脚本语言，语法类似 Rust/JavaScript。

### 基本语法

```rust
// 变量声明
let x = 42;                    // 可变变量
const PI = 3.14159;            // 常量

// 函数定义
fn add(a, b) {
    a + b                      // 最后一个表达式作为返回值
}

fn greet(name) {
    return `Hello, ${name}!`;  // 显式 return
}

// 字符串插值
let name = "伏羲";
let msg = `欢迎使用 ${name} 量化引擎`;

// 打印输出
print("Hello World");
print(`x = ${x}`);
```

### 数据类型

| 类型 | 示例 | 说明 |
|------|------|------|
| `int` | `42`, `-100` | 64 位整数 |
| `float` | `3.14`, `1.0e-5` | 64 位浮点数 |
| `bool` | `true`, `false` | 布尔值 |
| `string` | `"hello"`, `'world'` | 字符串 |
| `char` | `'A'`, `'中'` | 单个字符 |
| `array` | `[1, 2, 3]` | 数组 |
| `map` | `#{ a: 1, b: 2 }` | 对象/字典 |
| `()` | `()` | 空值 (类似 null) |
| `Decimal` | 交易 API 返回 | 高精度十进制 |

**类型转换：**

```rust
let x = 42;
let f = x.to_float();          // 整数 → 浮点
let i = f.to_int();            // 浮点 → 整数
let s = x.to_string();         // 转字符串
let d = x.to_decimal();        // 转 Decimal

// 类型检查
type_of(x) == "i64";
type_of(f) == "f64";
```

### 运算符

**算术运算：**

```rust
1 + 2                          // 加法
5 - 3                          // 减法
2 * 4                          // 乘法
10 / 3                         // 除法
10 % 3                         // 取模
2 ** 10                        // 幂运算 (1024)
```

**比较运算：**

```rust
a == b                         // 等于
a != b                         // 不等于
a > b                          // 大于
a >= b                         // 大于等于
a < b                          // 小于
a <= b                         // 小于等于
```

**逻辑运算：**

```rust
a && b                         // 逻辑与
a || b                         // 逻辑或
!a                             // 逻辑非
```

**位运算：**

```rust
a & b                          // 按位与
a | b                          // 按位或
a ^ b                          // 按位异或
!a                             // 按位取反
a << 2                         // 左移
a >> 2                         // 右移
```

**复合赋值：**

```rust
x += 1;                        // x = x + 1
x -= 1;                        // x = x - 1
x *= 2;                        // x = x * 2
x /= 2;                        // x = x / 2
x %= 3;                        // x = x % 3
```

### 控制流

**条件语句：**

```rust
if condition {
    // ...
} else if other_condition {
    // ...
} else {
    // ...
}

// 三元表达式
let result = if x > 0 { "positive" } else { "non-positive" };
```

**循环语句：**

```rust
// while 循环
while condition {
    // ...
    if should_stop { break; }
    if should_skip { continue; }
}

// for 循环
for item in array {
    print(item);
}

// 范围循环
for i in 0..10 {               // 0 到 9
    print(i);
}

for i in 0..=10 {              // 0 到 10 (包含)
    print(i);
}

// loop 无限循环
loop {
    if done { break; }
}
```

### 数组操作

```rust
let arr = [1, 2, 3, 4, 5];

// 基本操作
arr.len();                     // 长度: 5
arr.is_empty();                // 是否为空: false
arr[0];                        // 访问: 1
arr[-1];                       // 倒数第一个: 5

// 修改操作
arr.push(6);                   // 尾部添加
arr.pop();                     // 尾部弹出
arr.shift();                   // 头部弹出
arr.insert(0, 0);              // 指定位置插入
arr.remove(0);                 // 删除指定位置
arr.clear();                   // 清空

// 切片与搜索
arr.first();                   // 第一个元素
arr.last();                    // 最后一个元素
arr.get(1);                    // 安全访问
arr.contains(3);               // 是否包含
arr.index_of(3);               // 查找索引
3 in arr;                      // 是否存在

// 排序与翻转
arr.sort();                    // 升序排序
arr.sort(|a, b| b - a);        // 自定义排序
arr.reverse();                 // 翻转

// 函数式操作
arr.map(|x| x * 2);            // 映射: [2, 4, 6, 8, 10]
arr.filter(|x| x > 2);         // 过滤: [3, 4, 5]
arr.reduce(|a, b| a + b);      // 归约: 15
arr.some(|x| x > 3);           // 是否存在满足条件: true
arr.all(|x| x > 0);            // 是否全部满足: true
arr.for_each(|x| print(x));    // 遍历
```

### 对象 (Map)

```rust
let obj = #{
    name: "BTC-USDT",
    price: 50000.0,
    volume: 100
};

// 访问属性
obj.name;                      // "BTC-USDT"
obj["price"];                  // 50000.0

// 修改属性
obj.price = 51000.0;
obj["volume"] = 200;

// 常用方法
obj.len();                     // 属性数量
obj.is_empty();                // 是否为空
obj.keys();                    // 所有键
obj.values();                  // 所有值
obj.contains("name");          // 是否包含键
"name" in obj;                 // 是否存在
obj.remove("volume");          // 删除属性
obj.clear();                   // 清空
```

---

## 💰 Decimal 高精度计算

Rhai 内置 **rust_decimal** 支持，交易 API 中的价格和数量使用 `Decimal` 类型，确保金融计算精度。

### 创建 Decimal

```rust
// 从字符串解析
let d = parse_decimal("123.456");
type_of(d) == "decimal";

// 从整数/浮点转换
let d = 42.to_decimal();
let d = 3.14.to_decimal();

// 交易 API 返回 Decimal
let price = this.api.symbol("BTC-USDT").price;  // Decimal 类型
```

### 算术运算

```rust
let d = parse_decimal("2");

// Decimal 与整数互操作，结果为 Decimal
let x = d + 1;                 // Decimal + INT = Decimal
let x = 21 * d;                // INT * Decimal = Decimal
let x = d - 1;                 // 减法
let x = d / 2;                 // 除法
let x = d % 3;                 // 取模
let x = d ** 2;                // 幂运算

// 比较运算
d == 2;                        // Decimal == INT
d > 1;                         // Decimal > INT
10 < d * 10;                   // INT < Decimal
```

### 数学函数

| 函数 | 说明 |
|------|------|
| `abs(d)` | 绝对值 |
| `sign(d)` | 符号 (-1, 0, 1) |
| `is_zero(d)` | 是否为零 |
| `floor(d)` | 向下取整 |
| `ceiling(d)` | 向上取整 |
| `int(d)` | 取整数部分 |
| `fraction(d)` | 取小数部分 |
| `sqrt(d)` | 平方根 |
| `exp(d)` | 指数 e^d |
| `ln(d)` | 自然对数 |
| `log(d)` | 常用对数 |

### 舍入函数

```rust
let d = parse_decimal("3.14159");

round(d);                      // 四舍五入 → 3
round(d, 2);                   // 保留 2 位 → 3.14
round_up(d, 2);                // 向上舍入 → 3.15
round_down(d, 2);              // 向下舍入 → 3.14
round_half_up(d, 2);           // 四舍五入 (半数向上) → 3.14
round_half_down(d, 2);         // 四舍五入 (半数向下) → 3.14
floor(d);                      // 向下取整 → 3
ceiling(d);                    // 向上取整 → 4
int(d);                        // 整数部分 → 3
fraction(d);                   // 小数部分 → 0.14159
```

### 类型转换

```rust
let d = parse_decimal("123.45");

d.to_int();                    // Decimal → 整数 (截断)
d.to_float();                  // Decimal → 浮点
d.to_string();                 // Decimal → 字符串

// 其他类型转 Decimal
let x = 42;
x.to_decimal();                // 整数 → Decimal

let f = 3.14;
f.to_decimal();                // 浮点 → Decimal
```

### 比较函数

```rust
let a = parse_decimal("1.5");
let b = parse_decimal("2.5");

min(a, b);                     // 较小值 → 1.5
max(a, b);                     // 较大值 → 2.5
```

### 在交易中使用

```rust
let symbol = this.api.symbol("BTC-USDT");
let price = symbol.price;      // Decimal
let size = 0.1;                // 自动转换

// 计算
let total = price * size;
let fee = total * parse_decimal("0.001");

// 精度截断
let safe_size = symbol.trunc_size(size);
let safe_price = symbol.trunc_price(price);

// 比较
if price > 50000.0 {
    this.api.buy("BTC-USDT", safe_size);
}
```

> **注意**：Rhai 会自动将整数和浮点数与 Decimal 互操作，运算结果始终为 Decimal。

---

## ⌚ 时间处理

### Time 类型

```rust
// 构造
let t = now();                             // 当前时间
let t = to_time(1704067200000);            // 毫秒时间戳
let t = to_time("2024-01-01 08:00:00");    // 字符串解析
```

**属性：**

| 属性 | 类型 | 说明 |
|------|------|------|
| `year` | int | 年 |
| `month` | int | 月 (1-12) |
| `day` | int | 日 (1-31) |
| `hour` | int | 时 (0-23) |
| `minute` | int | 分 (0-59) |
| `second` | int | 秒 (0-59) |
| `millis` | int | 毫秒 (0-999) |
| `weekday` | int | 星期几 (0=周一, 6=周日) |
| `week` | int | ISO 周数 |
| `ordinal` | int | 一年中第几天 (1-366) |
| `quarter` | int | 季度 (1-4) |
| `timestamp` | int | 秒时间戳 |
| `timestamp_ms` | int | 毫秒时间戳 |

**方法：**

```rust
t.format("%Y-%m-%d %H:%M:%S"); // 格式化
t.to_string();                 // 转字符串
t.is_leap_year();              // 是否闰年
t.trunc(DAY);                  // 按天截断
t.trunc(HOUR);                 // 按小时截断
```

**运算：**

```rust
let t1 = now();
let t2 = t1 + DAY;             // 加一天
let t3 = t1 - HOUR * 2;        // 减两小时
let diff = t2 - t1;            // 差值，返回 Duration

// 比较
t1 == t2; t1 != t2;
t1 < t2;  t1 <= t2;
t1 > t2;  t1 >= t2;
```

### Duration 类型

**常量：**

| 常量 | 说明 |
|------|------|
| `DAY` | 1 天 |
| `HOUR` | 1 小时 |
| `MINUTE` | 1 分钟 |
| `SECOND` | 1 秒 |
| `MILLI` | 1 毫秒 |

**运算：**

```rust
let d = DAY * 2 + HOUR * 3;    // 组合
d + HOUR;                      // 加法
d - HOUR;                      // 减法
d * 2;                         // 乘整数
d / 2;                         // 除整数
-d;                            // 取负
```

**转换：**

```rust
d.to_days();                   // 转天数
d.to_hours();                  // 转小时
d.to_minutes();                // 转分钟
d.to_seconds();                // 转秒
d.to_millis();                 // 转毫秒
d.is_zero();                   // 是否为零
d.to_string();                 // 转字符串
```

---

## 🔥 交易 API

所有交易 API 通过 `this.api` 访问。

### 常量定义

**运行模式：**

| 常量 | 说明 |
|------|------|
| `BACKTEST` | 回测模式 |
| `MAINNET` | 主网模式 |

**订单类型：**

| 常量 | 说明 |
|------|------|
| `LIMIT` | 限价单 |
| `MARKET` | 市价单 |

**方向：**

| 常量 | 说明 |
|------|------|
| `LONG` | 做多 |
| `SHORT` | 做空 |

**买卖方向：**

| 常量 | 说明 |
|------|------|
| `BUY` | 买入 |
| `SELL` | 卖出 |

**订单状态：**

| 常量 | 说明 |
|------|------|
| `ORD_NEW` | 新建 |
| `ORD_PENDING` | 待处理 |
| `ORD_FILLED` | 已成交 |
| `ORD_CANCELING` | 取消中 |
| `ORD_CANCELED` | 已取消 |
| `ORD_REJECTED` | 已拒绝 |

**定时器：**

| 常量 | 说明 |
|------|------|
| `DAILY` | 每日 |
| `HOURLY` | 每小时 |
| `MINUTELY` | 每分钟 |
| `SECONDLY` | 每秒 |

### K 线与信号

```rust
// 获取当前回测时间 (毫秒时间戳)
let ts = this.api.time();

// 获取 K 线数据
let df = this.api.bars("BTC-USDT");       // 截止当前时间
let df = this.api.bars("BTC-USDT", true); // 包含未来数据

// 信号数据
let signals = this.api.signals();          // 获取信号
this.api.set_signals(signals_df);          // 设置信号
```

### 下单函数

**通用下单：**

```rust
// place_order(code, type, direction, side, size, [price])
let id = this.api.place_order("BTC-USDT", LIMIT, LONG, BUY, 0.1, 50000.0);
let id = this.api.place_order("BTC-USDT", MARKET, LONG, BUY, 0.1);

// 取消订单
this.api.cancel_order(order_id);
```

**快捷下单：**

```rust
// 做多开仓
this.api.buy("BTC-USDT", 0.1);             // 市价
this.api.buy("BTC-USDT", 0.1, 50000.0);    // 限价

// 做多平仓
this.api.sell("BTC-USDT", 0.1);            // 市价
this.api.sell("BTC-USDT", 0.1, 51000.0);   // 限价

// 做空开仓
this.api.short("BTC-USDT", 0.1);           // 市价
this.api.short("BTC-USDT", 0.1, 50000.0);  // 限价

// 做空平仓
this.api.cover("BTC-USDT", 0.1);           // 市价
this.api.cover("BTC-USDT", 0.1, 49000.0);  // 限价
```

### 资金查询

```rust
this.api.cash();               // 总现金
this.api.avail_cash();         // 可用现金
this.api.frozen_cash();        // 总冻结
this.api.order_frozen_cash();  // 订单冻结
this.api.pos_frozen_cash();    // 持仓冻结
this.api.upl();                // 未实现盈亏
this.api.equity();             // 权益
```

### 持仓管理

```rust
// 获取持仓
let positions = this.api.all_pos();        // 所有持仓
let p = this.api.pos("BTC-USDT");          // 按代码
let p = this.api.pos(0);                   // 按索引

// 持仓属性
p.code;                        // 合约代码
p.lever;                       // 杠杆倍数
p.long;                        // 多头持仓
p.short;                       // 空头持仓

// 方向持仓属性
p.long.price;                  // 多头均价
p.long.size;                   // 多头数量
p.short.price;                 // 空头均价
p.short.size;                  // 空头数量

// 可用/冻结数量
this.api.pos_frozen_size("BTC-USDT", LONG);
this.api.pos_avail_size("BTC-USDT", LONG);
```

### 订单管理

```rust
// 获取订单
let orders = this.api.open_orders();              // 所有未完成订单
let orders = this.api.open_orders("BTC-USDT");    // 按代码筛选
let order = this.api.order("order_id");           // 按 ID 获取

// 订单属性
order.id;                      // 订单 ID
order.code;                    // 合约代码
order.type_;                   // 订单类型
order.direction;               // 方向
order.side;                    // 买卖方向
order.price;                   // 价格
order.size;                    // 数量
order.filled;                  // 已成交数量
order.status;                  // 状态
order.time;                    // 下单时间 (毫秒时间戳)
```

### 合约信息

```rust
// 获取合约
let symbols = this.api.all_symbol();       // 所有合约
let s = this.api.symbol("BTC-USDT");       // 按代码
let s = this.api.symbol(0);                // 按索引

// 合约属性
s.code;                        // 交易对代码
s.price_tick;                  // 最小价格变动
s.size_tick;                   // 最小数量变动
s.min_size;                    // 最小交易数量
s.min_cash;                    // 最小交易金额
s.max_lever;                   // 最大杠杆
s.face_val;                    // 合约面值
s.mark_price;                  // 标记价格
s.price;                       // 最新价格
s.funding_rate;                // 资金费率

// 合约方法
s.trunc_size(0.12345);                     // 截断数量
s.trunc_price(50000.12345);                // 截断价格
s.cash_to_size(1000.0);                    // 金额转数量 (标记价)
s.cash_to_size(1000.0, 50000.0);           // 金额转数量 (指定价)
```

---

## 🧠 数据处理 (Polars)

引擎内置 **Polars** DataFrame 库，提供高性能数据分析能力。

### Polars 常量

**数据类型 (`DataType::`)：**

| 常量 | 说明 |
|------|------|
| `DataType::NULL` | 空值 |
| `DataType::BOOL` | 布尔 |
| `DataType::INT` | 64 位整数 |
| `DataType::FLOAT` | 64 位浮点 |
| `DataType::STR` | 字符串 |
| `DataType::TIME` | 日期时间 (Asia/Shanghai) |

**连接类型 (`JoinType::`)：**

| 常量 | 说明 |
|------|------|
| `JoinType::INNER` | 内连接 |
| `JoinType::LEFT` | 左连接 |
| `JoinType::RIGHT` | 右连接 |
| `JoinType::FULL` | 全连接 |
| `JoinType::SEMI` | 半连接 |
| `JoinType::ANTI` | 反连接 |
| `JoinType::CROSS` | 交叉连接 |

**空值填充策略 (`FullNull::`)：**

| 常量 | 说明 |
|------|------|
| `FullNull::MEAN` | 均值填充 |
| `FullNull::MIN` | 最小值填充 |
| `FullNull::MAX` | 最大值填充 |
| `FullNull::ZERO` | 零值填充 |
| `FullNull::ONE` | 1 填充 |

### Series

Series 是一维数据列。

**创建：**

```rust
// series(name, dtype, array)
let s = series("values", DataType::INT, [1, 2, 3, 4, 5]);
let s = series("floats", DataType::FLOAT, [1.0, 2.0, (), 4.0]);  // () 表示 null
let s = series("names", DataType::STR, ["Alice", "Bob"]);
let s = series("flags", DataType::BOOL, [true, false]);
let s = series("times", DataType::TIME, [1704067200000]);
```

**属性与统计：**

```rust
s.name();                      // 名称
s.len();                       // 长度
s.dtype();                     // 数据类型
s.is_empty();                  // 是否为空
s.null_count();                // 空值数量
s.sum();                       // 求和
s.mean();                      // 均值
s.min();                       // 最小值
s.max();                       // 最大值
s.std(1);                      // 标准差 (ddof=1)
s.variance(1);                 // 方差
s.median();                    // 中位数
```

**访问与切片：**

```rust
s.get(0);                      // 获取第 0 个元素
s.head();                      // 前 10 行
s.head(5);                     // 前 5 行
s.tail();                      // 后 10 行
s.tail(5);                     // 后 5 行
s.slice(1, 3);                 // 从索引 1 取 3 个
```

**变换操作：**

```rust
s.reverse();                   // 反转
s.shift(1);                    // 向下移动
s.shift(-1);                   // 向上移动
s.sort(false);                 // 升序
s.sort(true);                  // 降序
s.unique();                    // 去重
s.n_unique();                  // 唯一值数量
s.rename("new_name");          // 重命名
s.cast(DataType::FLOAT);       // 类型转换
```

**Null 处理：**

```rust
s.is_null();                   // 返回 bool Series
s.is_not_null();               // 返回 bool Series
s.drop_nulls();                // 删除空值
s.fill_null(FullNull::MEAN);   // 填充策略
```

**算术运算：**

```rust
a + b;                         // Series + Series
a - b; a * b; a / b;
s + 10;                        // Series + 标量
s * 2.0;
```

### DataFrame

DataFrame 是二维表格数据。

**创建：**

```rust
let df = dataframe([
    series("name", DataType::STR, ["Alice", "Bob"]),
    series("age", DataType::INT, [25, 30]),
    series("score", DataType::FLOAT, [85.5, 92.3])
]);
```

**属性：**

```rust
df.height();                   // 行数
df.width();                    // 列数
df.shape();                    // [行数, 列数]
df.columns();                  // 列名数组
df.dtypes();                   // 数据类型数组
df.is_empty();                 // 是否为空
```

**列操作：**

```rust
df.column("name");             // 获取列 → Series
df.select(["name", "age"]);    // 选择多列
df.with_column(new_series);    // 添加/替换列
df.drop("age");                // 删除列
df.rename("name", "username"); // 重命名列
```

**行操作：**

```rust
df.head();                     // 前 10 行
df.head(5);                    // 前 5 行
df.tail();                     // 后 10 行
df.slice(1, 3);                // 从索引 1 取 3 行
df.reverse();                  // 反转行
df.shift(1);                   // 行移动
```

**排序与过滤：**

```rust
df.sort(["score"], [false]);               // 升序
df.sort(["score"], [true]);                // 降序
df.sort(["age", "score"], [false, true]);  // 多列排序

let mask = series("m", DataType::BOOL, [true, false]);
df.filter(mask);                           // 按掩码过滤
```

**连接：**

```rust
df.vstack(other_df);                       // 垂直堆叠
df.join(other, ["id"], ["id"], JoinType::INNER);
df.join(other, ["id"], ["id"], JoinType::LEFT);
```

**Null 处理：**

```rust
df.null_count();               // 每列空值数
df.drop_nulls();               // 删除含空值行
```

**转换：**

```rust
let lf = df.lazy();            // 转 LazyFrame
```

### LazyFrame

LazyFrame 支持惰性计算，通过 `collect()` 触发执行。

**执行：**

```rust
let df = lf.collect();         // 执行并返回 DataFrame
```

**查询：**

```rust
lf.select([col("name"), col("score")]);    // 选择列
lf.filter(col("age").gt(lit(30)));         // 过滤
lf.with_column(expr);                      // 添加列
lf.with_columns([expr1, expr2]);           // 添加多列
```

**排序与切片：**

```rust
lf.sort(["score"], [false]);   // 排序
lf.slice(0, 10);               // 切片
lf.limit(5);                   // 前 5 行
lf.tail(5);                    // 后 5 行
```

**分组聚合：**

```rust
let result = df.lazy()
    .group_by([col("category")])
    .agg([
        col("value").sum().alias("total"),
        col("value").mean().alias("avg"),
        col("value").count().alias("cnt")
    ])
    .collect();
```

**连接：**

```rust
lf.join(other_lf, [col("id")], [col("id")], JoinType::INNER);
lf.join(other_lf, [col("id")], [col("id")], JoinType::LEFT);
```

**去重与 Null：**

```rust
lf.unique();                   // 去重
lf.drop_nulls();               // 删除空值行
lf.fill_null(lit(0));          // 填充空值
```

**聚合操作：**

```rust
lf.first();                    // 每列第一个
lf.last();                     // 每列最后一个
lf.max(); lf.min();            // 最值
lf.sum(); lf.mean();           // 求和/均值
lf.median();                   // 中位数
lf.std(1); lf.variance(1);     // 标准差/方差
lf.quantile(0.5);              // 分位数
lf.count();                    // 计数
lf.null_count();               // 空值数
```

**其他：**

```rust
lf.reverse();                  // 反转
lf.shift(1);                   // 移动
lf.cache();                    // 缓存
lf.with_row_index("idx");      // 添加行索引
lf.rename(["old"], ["new"]);   // 重命名
lf.describe_plan();            // 查询计划
```

### Expr 表达式

Expr 用于构建列操作表达式。

**基础：**

```rust
col("a");                      // 列引用
lit(42);                       // 字面量 (整数)
lit(3.14);                     // 字面量 (浮点)
lit("hello");                  // 字面量 (字符串)
lit(true);                     // 字面量 (布尔)

col("a").alias("new_name");    // 别名
```

**比较与逻辑：**

```rust
col("a").eq(col("b"));         // ==
col("a").neq(col("b"));        // !=
col("a").gt(lit(10));          // >
col("a").gte(lit(10));         // >=
col("a").lt(lit(10));          // <
col("a").lte(lit(10));         // <=

col("a").and(col("b"));        // 逻辑与
col("a").or(col("b"));         // 逻辑或
col("a").not();                // 逻辑非
```

**算术与数学：**

```rust
col("a") + col("b");           // 加
col("a") - col("b");           // 减
col("a") * col("b");           // 乘
col("a") / col("b");           // 除
col("a") % col("b");           // 取模
-col("a");                     // 取负
col("a").abs();                // 绝对值

col("a").sqrt();               // 平方根
col("a").pow(2.0);             // 幂运算
col("a").floor();              // 向下取整
col("a").ceil();               // 向上取整
col("a").round(2);             // 四舍五入
col("a").clip(0.0, 100.0);     // 裁剪范围
```

**聚合函数：**

```rust
col("a").sum();                // 求和
col("a").mean();               // 均值
col("a").min();                // 最小
col("a").max();                // 最大
col("a").median();             // 中位数
col("a").std(1);               // 标准差
col("a").variance(1);          // 方差
col("a").count();              // 计数
col("a").n_unique();           // 唯一值数
col("a").first();              // 第一个
col("a").last();               // 最后一个
col("a").quantile(0.5);        // 分位数
col("a").arg_min();            // 最小值索引
col("a").arg_max();            // 最大值索引
col("a").product();            // 乘积
```

**累积函数：**

```rust
col("a").cum_sum(false);       // 累积和
col("a").cum_prod(false);      // 累积积
col("a").cum_min(false);       // 累积最小
col("a").cum_max(false);       // 累积最大
col("a").cum_count(false);     // 累积计数
// 参数 true 表示反向计算
```

**滚动窗口：**

```rust
// rolling_*(window_size, min_periods)
col("a").rolling_sum(3, 1);    // 滚动求和
col("a").rolling_mean(3, 1);   // 滚动均值
col("a").rolling_std(3, 1);    // 滚动标准差
col("a").rolling_var(3, 1);    // 滚动方差
col("a").rolling_min(3, 1);    // 滚动最小
col("a").rolling_max(3, 1);    // 滚动最大
col("a").rolling_median(3, 1); // 滚动中位数
```

**移位与差分：**

```rust
col("a").shift(1);             // 向下移动
col("a").shift(-1);            // 向上移动
col("a").shift_and_fill(1, 0.0); // 移位并填充
col("a").diff(1);              // 一阶差分
```

**排序：**

```rust
col("a").sort(false);          // 升序
col("a").sort(true);           // 降序
col("a").reverse();            // 反转
col("a").arg_sort(false);      // 排序索引
```

**切片：**

```rust
col("a").head(5);              // 前 5 个
col("a").tail(5);              // 后 5 个
col("a").slice(1, 3);          // 切片
```

**Null 处理：**

```rust
col("a").is_null();            // 是否为空
col("a").is_not_null();        // 是否非空
col("a").is_nan();             // 是否 NaN
col("a").is_not_nan();         // 是否非 NaN
col("a").is_finite();          // 是否有限值
col("a").fill_null(lit(0));    // 填充空值
col("a").fill_nan(lit(0.0));   // 填充 NaN
col("a").drop_nulls();         // 删除空值
col("a").drop_nans();          // 删除 NaN
col("a").null_count();         // 空值数量
```

**去重：**

```rust
col("a").unique();             // 去重
col("a").unique_stable();      // 稳定去重
col("a").is_first_distinct();  // 首次出现
col("a").is_last_distinct();   // 最后出现
```

**高级操作：**

```rust
// 排名
col("a").rank("ordinal", false);  // 升序排名
col("a").rank("dense", true);     // 密集降序排名
// method: "average", "min", "max", "dense", "ordinal", "random"

// 窗口函数
col("a").sum().over([col("category")]);

// 成员判断
col("a").is_in(col("b"));
col("a").is_between(lit(0), lit(100), "both");
// closed: "left", "right", "both", "none"

// 插值
col("a").interpolate("linear");   // 线性插值
col("a").interpolate("nearest");  // 最近邻

// 其他
col("a").implode();            // 聚合为列表
col("a").explode();            // 展开列表
col("a").replace(lit(0), lit(-1)); // 替换值
col("a").sort_by([col("b")], [false]);
col("a").len();                // 列长度
```

**类型转换：**

```rust
col("a").cast(DataType::FLOAT);
col("a").cast(DataType::INT);
col("a").cast(DataType::STR);
```

**字符串操作 (`str_*`)：**

```rust
// 长度
col("s").str_len_bytes();      // 字节长度
col("s").str_len_chars();      // 字符长度

// 大小写
col("s").str_to_uppercase();
col("s").str_to_lowercase();

// 裁剪
col("s").str_strip_chars(" ");     // 去除首尾字符
col("s").str_strip_chars_start(" ");
col("s").str_strip_chars_end(" ");
col("s").str_strip_prefix("pre_");
col("s").str_strip_suffix("_suf");

// 搜索
col("s").str_contains("pat", true);   // 包含 (literal=true)
col("s").str_contains("pat.*", false); // 正则
col("s").str_starts_with("pre");
col("s").str_ends_with("suf");
col("s").str_find("pat", true);

// 替换
col("s").str_replace("old", "new", true);
col("s").str_replace_all("old", "new", true);

// 切片
col("s").str_slice(0, 5);
col("s").str_head(3);
col("s").str_tail(3);
col("s").str_reverse();

// 分割
col("s").str_split(",");
col("s").str_split_exact(",", 3);
col("s").str_splitn(",", 3);

// 提取
col("s").str_extract("(\\d+)", 1);
col("s").str_extract_all("\\d+");
col("s").str_count_matches("a", true);

// 转换
col("s").str_to_integer(10);   // 字符串转整数
```

**日期时间操作 (`dt_*`)：**

```rust
// 提取组件
col("t").dt_year();
col("t").dt_month();           // 1-12
col("t").dt_day();             // 1-31
col("t").dt_hour();            // 0-23
col("t").dt_minute();
col("t").dt_second();
col("t").dt_millisecond();
col("t").dt_microsecond();
col("t").dt_nanosecond();

// 周期
col("t").dt_weekday();         // 1=周一, 7=周日
col("t").dt_week();            // ISO 周数
col("t").dt_ordinal_day();     // 一年第几天
col("t").dt_quarter();         // 季度 1-4

// 判断
col("t").dt_is_leap_year();

// 时间戳
col("t").dt_timestamp("ms");   // 毫秒
col("t").dt_timestamp("us");   // 微秒
col("t").dt_timestamp("ns");   // 纳秒

// 截断与舍入
col("t").dt_truncate("1d");    // 按天截断
col("t").dt_truncate("1h");    // 按小时截断
col("t").dt_round("1d");

// 分离
col("t").dt_date();            // 日期部分
col("t").dt_time();            // 时间部分
```

**条件表达式 (When/Then/Otherwise)：**

```rust
// 简单条件
when(col("value").gt(lit(10)))
    .then(lit("high"))
    .otherwise(lit("low"))

// 链式条件
when(col("score").lt(lit(60)))
    .then(lit("F"))
    .when(col("score").lt(lit(70)))
    .then(lit("D"))
    .when(col("score").lt(lit(80)))
    .then(lit("C"))
    .otherwise(lit("B"))
```

### 全局函数

```rust
// 列引用
col("column_name");

// 字面量
lit(42);
lit(3.14);
lit("hello");
lit(true);

// 条件表达式
when(condition_expr);

// 选择所有列
all();
all().exclude(["a", "b"]);     // 排除指定列

// 选择指定列
cols(["a", "b", "c"]);

// LazyFrame 合并
concat_lazyframe([lf1, lf2, lf3]);
```

---

## 🎮 策略回调函数

策略脚本需要实现以下回调函数：

```rust
/// 策略启动时调用
fn on_start() {
    print("Strategy started");
}

/// 策略停止时调用
fn on_stop() {
    print("Strategy stopped");
}

/// K 线更新时调用
fn on_bar(code) {
    let df = this.api.bars(code);
    // 处理 K 线数据
}

/// 信号更新时调用
fn on_signal() {
    let signals = this.api.signals();
    // 处理信号
}

/// 定时器触发时调用
fn on_timer(timer, time) {
    if timer == DAILY {
        // 每日任务
    }
}

/// 订单状态更新时调用
fn on_order(order_id) {
    let order = this.api.order(order_id);
    print(`Order ${order_id} status: ${order.status}`);
}

/// 持仓变化时调用
fn on_position(code) {
    let pos = this.api.pos(code);
    // 处理持仓变化
}
```

---

## 🌟 完整示例

### 📊 双均线策略

```rust
fn on_start() {
    print("双均线策略启动");
}

fn on_bar(code) {
    let df = this.api.bars(code);
    
    // 计算移动平均线
    let result = df.lazy()
        .with_columns([
            col("close").rolling_mean(20, 1).alias("ma20"),
            col("close").rolling_mean(60, 1).alias("ma60")
        ])
        .collect();
    
    // 获取最新数据
    let last = result.tail(1);
    let ma20 = last.column("ma20").get(0);
    let ma60 = last.column("ma60").get(0);
    
    // 跳过无效数据
    if ma20 == () || ma60 == () {
        return;
    }
    
    let pos = this.api.pos(code);
    let symbol = this.api.symbol(code);
    
    // 金叉买入
    if ma20 > ma60 && pos.long.size == 0.0 {
        let cash = this.api.avail_cash();
        let size = symbol.cash_to_size(cash * 0.1);
        if size > 0.0 {
            this.api.buy(code, size);
            print(`买入 ${code} ${size}`);
        }
    }
    
    // 死叉卖出
    if ma20 < ma60 && pos.long.size > 0.0 {
        this.api.sell(code, pos.long.size);
        print(`卖出 ${code} ${pos.long.size}`);
    }
}

fn on_stop() {
    print("策略停止");
}
```

### 🔍 数据分析示例

```rust
// 创建数据
let df = dataframe([
    series("symbol", DataType::STR, ["BTC", "ETH", "BTC", "ETH"]),
    series("price", DataType::FLOAT, [42000.0, 2200.0, 43000.0, 2300.0]),
    series("volume", DataType::FLOAT, [100.0, 500.0, 150.0, 600.0])
]);

// 分组聚合
let summary = df.lazy()
    .group_by([col("symbol")])
    .agg([
        col("price").mean().alias("avg_price"),
        col("volume").sum().alias("total_volume"),
        col("price").max().alias("high"),
        col("price").min().alias("low")
    ])
    .with_column(
        (col("high") - col("low")).alias("range")
    )
    .sort(["total_volume"], [true])
    .collect();

print(summary);
```

### ⏳ 时间过滤

```rust
let df = this.api.bars("BTC-USDT");

let start = to_time("2024-01-01 00:00:00");
let end = start + DAY * 7;

let filtered = df.lazy()
    .filter(
        col("time").gte(lit(start.timestamp_ms))
        .and(col("time").lt(lit(end.timestamp_ms)))
    )
    .collect();
```

---

## 💡 注意事项

1. **Decimal 精度** — 交易相关的价格和数量使用 Decimal 类型，Rhai 自动从浮点数转换
2. **Null 值** — 在 Rhai 中用 `()` 表示空值
3. **惰性计算** — LazyFrame 操作是惰性的，需调用 `collect()` 执行
4. **时区** — `DataType::TIME` 默认使用 `Asia/Shanghai` 时区
5. **错误处理** — 大部分函数在出错时抛出运行时错误
6. **Gas 限制** — 脚本有执行步数限制，避免无限循环
