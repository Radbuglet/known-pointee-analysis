; ModuleID = 'input_ll'
source_filename = "input_ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32"
target triple = "arm64-apple-macosx15.0.0"

@.str = private constant [10 x i8] c"panicked!\00", align 1
@.str.1 = private constant [29 x i8] c"bad analysis at location %u\0A\00", align 1

; Function Attrs: cold mustprogress nofree noinline noreturn ssp uwtable(sync)
define void @_Z5panicv() #0 {
entry_block2v1:
  %v1 = call i32 @puts(ptr @.str)
  call void @exit(i32 1) #7
  unreachable
}

; Function Attrs: nofree nounwind
declare i32 @puts(ptr) #1

; Function Attrs: nofree noreturn
declare void @exit(i32) #2

; Function Attrs: mustprogress nofree noinline norecurse nosync nounwind ssp willreturn memory(none) uwtable(sync)
define void @_Z21arbitrary_side_effectv() #3 {
entry_block3v1:
  ret void
}

; Function Attrs: mustprogress nofree noinline nounwind ssp uwtable(sync)
define void @meow_assert(i1 %0, i32 %1) #4 {
entry_block4v1:
  br i1 %0, label %block6v1, label %block5v1

block5v1:                                         ; preds = %entry_block4v1
  %v7 = call i32 (ptr, ...) @printf(ptr @.str.1, i32 %1)
  br label %block6v1

block6v1:                                         ; preds = %block5v1, %entry_block4v1
  ret void
}

; Function Attrs: nofree nounwind
declare i32 @printf(ptr, ...) #1

; Function Attrs: mustprogress nofree ssp uwtable(sync)
define void @_Z4meowPjS_(ptr %0, ptr %1) #5 {
entry_block7v1:
  %v10 = load i32, ptr %0, align 4, !tbaa !10
  %v12 = icmp eq i32 %v10, 1
  br i1 %v12, label %block9v1, label %block8v1

block8v1:                                         ; preds = %entry_block7v1
  call void @_Z5panicv() #8
  unreachable

block9v1:                                         ; preds = %entry_block7v1
  store i32 2, ptr %0, align 4, !tbaa !10
  %v15 = load i32, ptr %1, align 4, !tbaa !10
  %v17 = icmp eq i32 %v15, 3
  br i1 %v17, label %block11v1, label %block10v1

block10v1:                                        ; preds = %block9v1
  call void @_Z5panicv() #8
  unreachable

block11v1:                                        ; preds = %block9v1
  store i32 1, ptr %0, align 4, !tbaa !10
  store i32 3, ptr %1, align 4, !tbaa !10
  %v19 = load i32, ptr %0, align 4, !tbaa !10
  %v20 = icmp eq i32 %v19, 1
  br i1 %v20, label %block13v1, label %block12v1

block12v1:                                        ; preds = %block11v1
  call void @meow_assert(i1 false, i1 false)
  call void @_Z5panicv() #8
  call void @meow_assert(i1 false, i1 true)
  unreachable

block13v1:                                        ; preds = %block11v1
  store i32 2, ptr %0, align 4, !tbaa !10
  %v22 = load i32, ptr %1, align 4, !tbaa !10
  %v23 = icmp eq i32 %v22, 3
  br i1 %v23, label %block15v1, label %block14v1

block14v1:                                        ; preds = %block13v1
  call void @meow_assert(i1 false, i1 true)
  call void @_Z5panicv() #8
  call void @meow_assert(i1 false, i1 true)
  unreachable

block15v1:                                        ; preds = %block13v1
  store i32 1, ptr %0, align 4, !tbaa !10
  store i32 3, ptr %1, align 4, !tbaa !10
  ret void
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind ssp willreturn memory(none) uwtable(sync)
define i32 @main() #6 {
entry_block16v1:
  ret i32 0
}

attributes #0 = { cold mustprogress nofree noinline noreturn ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #1 = { nofree nounwind "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #2 = { nofree noreturn "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #3 = { mustprogress nofree noinline norecurse nosync nounwind ssp willreturn memory(none) uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #4 = { mustprogress nofree noinline nounwind ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #5 = { mustprogress nofree ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #6 = { mustprogress nofree norecurse nosync nounwind ssp willreturn memory(none) uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #7 = { cold noreturn }
attributes #8 = { noreturn }

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
