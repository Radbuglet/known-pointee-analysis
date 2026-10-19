; ModuleID = 'input_ll'
source_filename = "input_ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-n32:64-S128-Fn32"
target triple = "arm64-apple-macosx15.0.0"

@.str = private constant [29 x i8] c"bad analysis at location %u\0A\00", align 1
@.str.1 = private constant [27 x i8] c"I'm allegedly unreachable!\00", align 1
@.str.2 = private constant [15 x i8] c"*foo = 0x%llx\0A\00", align 1
@.str.3 = private constant [7 x i8] c"demo 1\00", align 1
@.str.4 = private constant [7 x i8] c"demo 2\00", align 1

; Function Attrs: mustprogress nofree noinline nounwind ssp uwtable(sync)
define void @meow_assert(i1 %0, i32 %1) #0 {
entry_block2v1:
  br i1 %0, label %block4v1, label %block3v1

block3v1:                                         ; preds = %entry_block2v1
  %v3 = call i32 (ptr, ...) @printf(ptr @.str, i32 %1)
  br label %block4v1

block4v1:                                         ; preds = %block3v1, %entry_block2v1
  ret void
}

; Function Attrs: nofree nounwind
declare i32 @printf(ptr, ...) #1

; Function Attrs: mustprogress nofree noinline nounwind ssp uwtable(sync)
define void @_Z6demo_1PyPh(ptr %0, ptr %1) #0 {
entry_block5v1:
  store i64 1, ptr %0, align 8, !tbaa !10
  store i8 1, ptr %1, align 1, !tbaa !12
  %v8 = load i64, ptr %0, align 8, !tbaa !10
  %v37 = icmp eq i64 %v8, 1
  call void @meow_assert(i1 %v37, i32 0)
  %v9 = icmp eq i64 %v8, 1
  %v41 = icmp eq i1 %v9, true
  call void @meow_assert(i1 %v41, i32 1)
  br i1 %v9, label %block7v1, label %block6v1

block6v1:                                         ; preds = %entry_block5v1
  call void @meow_assert(i1 false, i32 2)
  %v11 = call i32 @puts(ptr @.str.1)
  call void @meow_assert(i1 false, i32 3)
  br label %block7v1

block7v1:                                         ; preds = %block6v1, %entry_block5v1
  ret void
}

; Function Attrs: nofree nounwind
declare i32 @puts(ptr) #1

; Function Attrs: mustprogress nofree noinline nounwind ssp uwtable(sync)
define void @_Z6demo_2PyPh(ptr %0, ptr %1) #0 {
entry_block8v1:
  store i64 1, ptr %0, align 8, !tbaa !10
  store i8 1, ptr %1, align 1, !tbaa !12
  %v16 = load i64, ptr %0, align 8, !tbaa !10
  %v51 = icmp eq i64 %v16, 1
  call void @meow_assert(i1 %v51, i32 0)
  %v18 = call i32 (ptr, ...) @printf(ptr @.str.2, i64 %v16)
  ret void
}

; Function Attrs: mustprogress nofree norecurse nounwind ssp uwtable(sync)
define i32 @main() #2 {
entry_block9v1:
  %v20 = alloca i64, align 8
  %v21 = alloca i64, align 8
  %v23 = call i32 @puts(ptr @.str.3)
  call void @llvm.lifetime.start.p0(ptr %v20) #4
  store i64 1, ptr %v20, align 8, !tbaa !10
  %v26 = getelementptr inbounds nuw i8, ptr %v20, i64 1
  call void @_Z6demo_1PyPh(ptr %v20, ptr %v26)
  %v29 = call i32 @puts(ptr @.str.4)
  call void @llvm.lifetime.start.p0(ptr %v21) #4
  store i64 1, ptr %v21, align 8, !tbaa !10
  %v31 = getelementptr inbounds nuw i8, ptr %v21, i64 1
  call void @_Z6demo_2PyPh(ptr %v21, ptr %v31)
  call void @llvm.lifetime.end.p0(ptr %v21) #4
  call void @llvm.lifetime.end.p0(ptr %v20) #4
  ret i32 0
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(ptr captures(none)) #3

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(ptr captures(none)) #3

attributes #0 = { mustprogress nofree noinline nounwind ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #1 = { nofree nounwind "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #2 = { mustprogress nofree norecurse nounwind ssp uwtable(sync) "frame-pointer"="non-leaf-no-reserve" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="apple-m1" "target-features"="+aes,+altnzcv,+ccdp,+ccidx,+ccpp,+complxnum,+crc,+dit,+dotprod,+flagm,+fp-armv8,+fp16fml,+fptoint,+fullfp16,+jsconv,+lse,+neon,+pauth,+perfmon,+predres,+ras,+rcpc,+rdm,+sb,+sha2,+sha3,+specrestrict,+ssbs,+v8.1a,+v8.2a,+v8.3a,+v8.4a,+v8a" "tune-cpu"="apple-m5" }
attributes #3 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { nounwind }

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
!10 = !{!11, !11, i64 0}
!11 = !{!"long long", !8, i64 0}
!12 = !{!8, !8, i64 0}
