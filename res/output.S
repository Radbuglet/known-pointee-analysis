	.build_version macos, 15, 0	sdk_version 15, 5
	.section	__TEXT,__text,regular,pure_instructions
	.globl	__Z5panicv                      ; -- Begin function _Z5panicv
	.p2align	2
__Z5panicv:                             ; @_Z5panicv
	.cfi_startproc
; %bb.0:                                ; %entry_block2v1
	stp	x29, x30, [sp, #-16]!           ; 16-byte Folded Spill
	mov	x29, sp
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
Lloh0:
	adrp	x0, l_.str@PAGE
Lloh1:
	add	x0, x0, l_.str@PAGEOFF
	bl	_puts
	mov	w0, #1                          ; =0x1
	bl	_exit
	.loh AdrpAdd	Lloh0, Lloh1
	.cfi_endproc
                                        ; -- End function
	.globl	__Z21arbitrary_side_effectv     ; -- Begin function _Z21arbitrary_side_effectv
	.p2align	2
__Z21arbitrary_side_effectv:            ; @_Z21arbitrary_side_effectv
	.cfi_startproc
; %bb.0:                                ; %entry_block3v1
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_meow_assert                    ; -- Begin function meow_assert
	.p2align	2
_meow_assert:                           ; @meow_assert
	.cfi_startproc
; %bb.0:                                ; %entry_block4v1
                                        ; kill: def $w1 killed $w1 def $x1
	tbnz	w0, #0, LBB2_2
; %bb.1:                                ; %block5v1
	sub	sp, sp, #32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	add	x29, sp, #16
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	str	x1, [sp]
Lloh2:
	adrp	x0, l_.str.1@PAGE
Lloh3:
	add	x0, x0, l_.str.1@PAGEOFF
	bl	_printf
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #32
LBB2_2:                                 ; %block6v1
	ret
	.loh AdrpAdd	Lloh2, Lloh3
	.cfi_endproc
                                        ; -- End function
	.globl	__Z4meowPjS_                    ; -- Begin function _Z4meowPjS_
	.p2align	2
__Z4meowPjS_:                           ; @_Z4meowPjS_
	.cfi_startproc
; %bb.0:                                ; %entry_block7v1
	stp	x29, x30, [sp, #-16]!           ; 16-byte Folded Spill
	mov	x29, sp
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	ldr	w8, [x0]
	cmp	w8, #1
	b.ne	LBB3_5
; %bb.1:                                ; %block9v1
	mov	w8, #2                          ; =0x2
	str	w8, [x0]
	ldr	w8, [x1]
	cmp	w8, #3
	b.ne	LBB3_5
; %bb.2:                                ; %block11v1
	mov	w8, #1                          ; =0x1
	str	w8, [x0]
	mov	w8, #3                          ; =0x3
	str	w8, [x1]
	ldr	w8, [x0]
	cmp	w8, #1
	b.ne	LBB3_6
; %bb.3:                                ; %block13v1
	mov	w8, #2                          ; =0x2
	str	w8, [x0]
	ldr	w8, [x1]
	cmp	w8, #3
	b.ne	LBB3_6
; %bb.4:                                ; %block15v1
	mov	w8, #1                          ; =0x1
	str	w8, [x0]
	mov	w8, #3                          ; =0x3
	str	w8, [x1]
	ldp	x29, x30, [sp], #16             ; 16-byte Folded Reload
	ret
LBB3_5:                                 ; %block8v1
	bl	__Z5panicv
LBB3_6:                                 ; %block12v1
	bl	_meow_assert
	bl	__Z5panicv
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:                                ; %entry_block16v1
	mov	w0, #0                          ; =0x0
	ret
	.cfi_endproc
                                        ; -- End function
	.section	__TEXT,__const
l_.str:                                 ; @.str
	.asciz	"panicked!"

l_.str.1:                               ; @.str.1
	.asciz	"bad analysis at location %u\n"

.subsections_via_symbols
