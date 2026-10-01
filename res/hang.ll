define void @f4(ptr %p, ptr %q, i1 %sel, i32 %u) {
entry:
  store i32 1, ptr %p, align 4
  br label %H
H:
  br i1 %sel, label %A, label %B
A:
  store i32 %u, ptr %p, align 4
  store i32 1, ptr %q, align 4
  %v1 = load i32, ptr %p, align 4
  br label %J
B:
  store i32 1, ptr %q, align 4
  %v2 = load i32, ptr %p, align 4
  br label %J
J:
  %x = phi i32 [ %v1, %A ], [ %v2, %B ]
  %cmp = icmp eq i32 %x, 2
  br i1 %cmp, label %T, label %F
T:
  store i32 1, ptr %p, align 4
  store i32 3, ptr %q, align 4
  br label %H
F:
  ret void
}
