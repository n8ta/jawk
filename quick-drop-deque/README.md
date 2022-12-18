# Quick Drop Deque
> quick drop deck


The stdlib VecDeque has no way to remove N elements from the front of the deque in O(1) time. 
This may be because they need to handle types that need `Drop` functions but I don't need to handle that
since I'm just using bytes. This deque is mostly copied from the stdlib with 1 additional method `drop_front`.
It is specialized for u8 but could be easily modified to handle any non-drop type.

# 💣 Warning 💣
I hacked a copy of RawVec out of the stdlib in about an hour and a half to be specific to u8 and use the global allocator. This was needed 
for the deque.
It's certainly got some memory safety problem in there so use it at your own risk.

# LICENSE
MIT

This project is used within jawk. The MIT license applies only to quick-drop-deque please check other project directories for their
licenses which may be different.