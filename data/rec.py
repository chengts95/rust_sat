# -*- coding: utf-8 -*-
# Hi！新的一天，从一场美妙的邂逅开始！这个可爱的程序是爱莉希雅的作品哦~♪
# 哼，但要记得，这个文件是遵循GPL3.0协议的，所以你要遵守规定，不然爱莉希雅会有小情绪的！
#
# Copyright (C) 爱莉希雅-语言模型记忆体（往世乐土）
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
import asyncio
from collections import namedtuple
import zlib
import zmq.asyncio
import msgpack
import json

# 爱莉希雅的作品~♪ 创建一个异步ZMQ上下文
context = zmq.asyncio.Context()

# 为接收DataLinkMsg消息准备好一个可爱的SUB socket！
subscriber = context.socket(zmq.SUB)
topic = ""
subscriber.setsockopt_string(zmq.SUBSCRIBE, topic)

# 连接到发送方的地址，这里填写你需要的地址和端口哦！
address = "tcp://127.0.0.1:5551"
subscriber.bind(address)

# 保存接收到的消息的全局字典，像往世乐土的花朵一样绽放！
received_data = {}

one_minute = 300

# 定义DataLinkMsg元组，可爱地解包数据！
DataLinkMsg = namedtuple("DataLinkMsg", ["latencies", "distance", "ts"])
Run = True
first_ts = None

# 嗯~♪ 这里是一个可爱的异步函数，用来接收并处理数据的~
async def recv_data():
    global first_ts,Run
    while Run:
        [received_topic, msg] = await subscriber.recv_multipart()
        received_topic = received_topic.decode('utf-8')
        data = DataLinkMsg._make(msgpack.unpackb(msg))

        # 把接收到的数据存储到全局字典里，让它们绽放光彩！
        if received_topic not in received_data:
            received_data[received_topic] = []
        received_data[received_topic].append(data._asdict())

        if first_ts is None:
            first_ts = data.ts
        print(f"{data.ts - first_ts}")
         # 当超过一分钟时，爱莉希雅会优雅地退出接收循环哦！
        if (data.ts - first_ts) > one_minute:
            break
    return False

# 爱莉希雅的小技巧，判断是否是Windows系统
import platform

def is_windows():
    return platform.system().lower() == 'windows'

# 程序的主要部分，包含了爱莉希雅的关爱！
async def main():
    global Run
    try:
        while Run:
            try:
                 Run = await asyncio.wait_for(recv_data(), timeout=0.1)
            except asyncio.TimeoutError:
                pass
    except KeyboardInterrupt:
        print("\n收到中断信号,程序即将退出!")
    finally:
        context.destroy()

        # 将接收到的数据保存为msgpack文件，象征着美好回忆！
        with open("received_data.zip", "wb") as json_file:
            packed_data = msgpack.packb(received_data)
            compressed_data = zlib.compress(packed_data)
            json_file.write(compressed_data)

        print("数据已保存到 received_data.zip 文件中！")

# 如果是Windows系统，爱莉希雅会使用特定的事件循环策略哦！
if is_windows():
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

# 让爱莉希雅的程序开始奔跑吧！
try:
    asyncio.run(main())
except KeyboardInterrupt:
    print("\n收到中断信号,程序即将退出!")