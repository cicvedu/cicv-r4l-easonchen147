<p align="center">
  <p align="center">
   EasonChen147 Rust For Linux 作业报告
</p>


## 目录

- [作业1-编译Linux内核](#作业1-编译linux内核)
- [作业2-对Linux内核进行一些配置](#作业2-对linux内核进行一些配置)
- [作业3-使用rust编写一个简单的内核模块并运行](#作业3-使用rust编写一个简单的内核模块并运行)
- [作业4-为e1000网卡驱动添加remove代码](#作业4-为e1000网卡驱动添加remove代码)
- [作业5-注册字符设备](#作业5-注册字符设备)

## 作业1-编译linux内核

1.初始化 x86_64 架构的默认配置文件

> make x86_64_defconfig

2.进入可视化界面并选择linux内核

> make LLVM=1 menuconfig

![1](imgs/1-1.png)

3.开始编译内核

> make LLVM=1 -j$(nproc) //-j 指定并行编译任务数量 $(nproc) 指定核心数

![2](imgs/1-2.png)



## 作业2-对linux内核进行一些配置

1.去掉内核自带的网卡驱动，排除即可

![1](imgs/2-1.png)

2.重新编译内核

![2](imgs/2-2.png)

3.内核已经重新编译成功

![3](imgs/2-3.png)

4.重新进入qemu 并进行设置

安装rust 的驱动，设置网络
![4](imgs/2-4.png)

![5](imgs/2-5.png)



## 作业3-使用rust编写一个简单的内核模块并运行

1.创建 rust_helloworld.rs 文件，并输入内容

![1](imgs/3-1.png)

2.修改Makfilee文件

![2](imgs/3-2.png)

3.修改Kconfig 文件

![3](imgs/3-3.png)

4.重新编译内核

![4](imgs/3-4.png)

![5](imgs/3-5.png)

![6](imgs/3-6.png)

5.进入系统安装模块

![7](imgs/3-7.png)

![8](imgs/3-8.png)

![9](imgs/3-9.png)



## 作业4-为e1000网卡驱动添加remove代码

1.安装内核模块并配置

![1](imgs/4-1.png)

2.卸载模块

![2](imgs/4-2.png)

3.重新安装模块，正常ping通

![3](imgs/4-3.png)

![4](imgs/4-4.png)

4.调整代码位置，实现思路是尽可能仿照e1000_main.c的资源释放来实现

资源声明补充
```rust

// 保存需要释放的资源在PrvData
struct E1000DrvPrvData {
    bars: i32,
    irq: u32,
    _netdev_reg: net::Registration<NetDevice>,
    dev_ptr: *mut bindings::pci_dev,
    e1000_hw_ops: Arc<E1000Ops>,
    _irq_handler: AtomicPtr<kernel::irq::Registration<E1000InterruptHandler>>,
}

// 相关资源填充，补充赋值需要释放的资源
Ok(Box::try_new(
        E1000DrvPrvData {
            bars,
            irq: irq,
            // Must hold this registration, or the device will be removed.
            _netdev_reg: netdev_reg,
            dev_ptr: dev.to_ptr(),
            e1000_hw_ops: Arc::try_new(e1000_hw_ops)?,
            _irq_handler: AtomicPtr::new(core::ptr::null_mut()),
        }
    )?)
```

释放资源
```rust
// 资源句柄释放
impl driver::DeviceRemoval for E1000DrvPrvData {
    fn device_remove(&self) {
        pr_info!("Rust for linux e1000 driver demo (device_remove)\n");

        drop(&self._irq_handler.load(core::sync::atomic::Ordering::Relaxed));
        drop(&self._netdev_reg);
    }
}
```

把相关需要释放的资源全部在remove方法里进行释放，避免下次挂载模块驱动时，出现内存被占用的情况
```rust
// 完善 remove 方法，进行资源释放
fn remove(data: &Self::Data) {
    pr_info!("Rust for linux e1000 driver demo (remove)\n");

    let netdev = data._netdev_reg.dev_get();
    let bars = data.bars;
    let pci_dev_ptr = data.dev_ptr;

    data.e1000_hw_ops.as_arc_borrow().e1000_reset_hw();
    netdev.netif_carrier_off();
    netdev.netif_stop_queue();

    unsafe {
        bindings::pci_clear_master(pci_dev_ptr);
        bindings::pci_release_selected_regions(pci_dev_ptr, bars);
        bindings::pci_disable_device(pci_dev_ptr);
    }
}
```

5.暂时还没能卸载模块后重装，还需要继续释放完整



## 作业5-注册字符设备

1.代码改动点

写入方法的完善，补充了对全局buffer的写入操作
```rust
 fn write(_this: &Self, _file: &file::File, _reader: &mut impl kernel::io_buffer::IoBufferReader, _offset: u64) -> Result<usize> {
        // empty return , nothing to do
        if _reader.is_empty() {
            return Ok(0);
        }

        let mut buf = _this.inner.lock();

        // check data len with max buf size
        let mut data_len = _reader.len();
        if data_len > GLOBALMEM_SIZE {
            data_len = GLOBALMEM_SIZE
        }

        // pr_info!("offset: {} data len: {}\n", _offset, data_len);

        _reader.read_slice(&mut buf[..data_len])?;

        Ok(data_len)
    }
```

读取方法的完善，把全局buffer缓冲区里的数据读取操作
```rust
 fn read(_this: &Self, _file: &file::File, _writer: &mut impl kernel::io_buffer::IoBufferWriter, _offset: u64) -> Result<usize> {
        let buf = &mut _this.inner.lock();

        // check _offset out of max buf size
        if _offset as usize >= GLOBALMEM_SIZE {
            return Ok(0);
        }

        _writer.write_slice(&buf[_offset as usize..])?;

        Ok(buf.len())
    }
```

2.重新编译linux内核，然后进入内核

![1](imgs/5-1.png)

3.安装字符设备驱动验证功能
![2](imgs/5-2.png)

4.作业问题回答

```rust
字符设备/dev/cicv是怎么创建的？它的设备号是多少？它是如何与我们写的字符设备驱动关联上的？

在脚本 build_image.sh 里，进行字符设备的创建及ID指定

echo "mknod /dev/cicv c 248 0" >> etc/init.d/rcS   // 命令的意思是指定了 /dev/cicv 作为字符设备，并切指定设备ID为 248

字符设备的关联其实是通过代码里的 Registration 进行关联，因为其使用了 chrdev 的驱动模块，固然内核会自动寻找对应类型，也就是字符设备来进行关联

代码片段：
let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;
chrdev_reg.as_mut().register::<RustFile>()?;
chrdev_reg.as_mut().register::<RustFile>()?;

```
