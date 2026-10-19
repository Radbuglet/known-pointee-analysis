; ModuleID = 'input_ll'
source_filename = "input_ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32"
target triple = "arm64-apple-macosx15.0.0"

; Function Attrs: mustprogress ssp uwtable(sync)
define void @_Z4meowPjS_(ptr %0, ptr %1) #0 {
entry_block2v1:
  %v2 = load i32, ptr %0, align 4, !tbaa !10
  %v4 = icmp eq i32 %v2, 1
  br i1 %v4, label %block4v1, label %block3v1

block3v1:                                         ; preds = %entry_block2v1
  call void @_Z5panicv() #2
  unreachable

block4v1:                                         ; preds = %entry_block2v1
  store i32 2, ptr %0, align 4, !tbaa !10
  %v7 = load i32, ptr %1, align 4, !tbaa !10
  %v9 = icmp eq i32 %v7, 3
  br i1 %v9, label %block6v1, label %block5v1

block5v1:                                         ; preds = %block4v1
  call void @_Z5panicv() #2
  unreachable

block6v1:                                         ; preds = %block4v1
  store i32 1, ptr %0, align 4, !tbaa !10
  store i32 3, ptr %1, align 4, !tbaa !10
  %v11 = load i32, ptr %0, align 4, !tbaa !10
  %v12 = icmp eq i32 %v11, 1
  br i1 %v12, label %block8v1, label %block7v1

block7v1:                                         ; preds = %block6v1
  call void @_Z5panicv() #2
  unreachable

block8v1:                                         ; preds = %block6v1
  store i32 2, ptr %0, align 4, !tbaa !10
  %v14 = load i32, ptr %1, align 4, !tbaa !10
  %v15 = icmp eq i32 %v14, 3
  br i1 %v15, label %block10v1, label %block9v1

block9v1:                                         ; preds = %block8v1
  call void @_Z5panicv() #2
  unreachable

block10v1:                                        ; preds = %block8v1
  store i32 1, ptr %0, align 4, !tbaa !10
  store i32 3, ptr %1, align 4, !tbaa !10
  ret void
}

; Function Attrs: noreturn
declare void @_Z5panicv() #1

attributes #0 = { mustprogress ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #1 = { noreturn "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #2 = { noreturn }

!llvm.module.flags = !{!0, !1, !2, !3}
!llvm.ident = !{!4}
!llvm.errno.tbaa = !{!5}

!0 = !{i32 2, !"SDK Version", [2 x i32] [i32 15, i32 5]}
!1 = !{i32 8, !"PIC Level", i32 2}
!2 = !{i32 7, !"uwtable", i32 1}
!3 = !{i32 7, !"frame-pointer", i32 4}
!4 = !{!"Homebrew clang version 23.1.1"}
!5 = !{!6, !7, i64 0}
!6 = !{!"__libc_errno", !7, i64 0}
!7 = !{!"int", !8, i64 0}
!8 = !{!"omnipotent char", !9, i64 0}
!9 = !{!"Simple C++ TBAA"}
!10 = !{!7, !7, i64 0}
