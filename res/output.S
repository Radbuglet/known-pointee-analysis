	.build_version macos, 15, 0	sdk_version 15, 5
	.section	__TEXT,__text,regular,pure_instructions
	.globl	_meow_assert                    ; -- Begin function meow_assert
	.p2align	2
_meow_assert:                           ; @meow_assert
	.cfi_startproc
; %bb.0:                                ; %entry_block2v1
                                        ; kill: def $w1 killed $w1 def $x1
	tbnz	w0, #0, LBB0_2
; %bb.1:                                ; %block3v1
	sub	sp, sp, #32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	add	x29, sp, #16
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	str	x1, [sp]
Lloh0:
	adrp	x0, l_.str@PAGE
Lloh1:
	add	x0, x0, l_.str@PAGEOFF
	bl	_printf
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #32
LBB0_2:                                 ; %block4v1
	ret
	.loh AdrpAdd	Lloh0, Lloh1
	.cfi_endproc
                                        ; -- End function
	.globl	__Z6demo_1PyPh                  ; -- Begin function _Z6demo_1PyPh
	.p2align	2
__Z6demo_1PyPh:                         ; @_Z6demo_1PyPh
	.cfi_startproc
; %bb.0:                                ; %entry_block5v1
	stp	x20, x19, [sp, #-32]!           ; 16-byte Folded Spill
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	add	x29, sp, #16
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	mov	w8, #1                          ; =0x1
	str	x8, [x0]
	strb	w8, [x1]
	ldr	x20, [x0]
	cmp	x20, #1
	cset	w19, eq
	mov	x0, x19
	mov	w1, #0                          ; =0x0
	bl	_meow_assert
	mov	x0, x19
	mov	w1, #1                          ; =0x1
	bl	_meow_assert
	cmp	x20, #1
	b.eq	LBB1_2
; %bb.1:                                ; %block6v1
	mov	w0, #0                          ; =0x0
	mov	w1, #2                          ; =0x2
	bl	_meow_assert
Lloh2:
	adrp	x0, l_.str.1@PAGE
Lloh3:
	add	x0, x0, l_.str.1@PAGEOFF
	bl	_puts
	mov	w0, #0                          ; =0x0
	mov	w1, #3                          ; =0x3
	bl	_meow_assert
LBB1_2:                                 ; %block7v1
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp], #32             ; 16-byte Folded Reload
	ret
	.loh AdrpAdd	Lloh2, Lloh3
	.cfi_endproc
                                        ; -- End function
	.globl	__Z6demo_2PyPh                  ; -- Begin function _Z6demo_2PyPh
	.p2align	2
__Z6demo_2PyPh:                         ; @_Z6demo_2PyPh
	.cfi_startproc
; %bb.0:                                ; %entry_block8v1
	sub	sp, sp, #48
	stp	x20, x19, [sp, #16]             ; 16-byte Folded Spill
	stp	x29, x30, [sp, #32]             ; 16-byte Folded Spill
	add	x29, sp, #32
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	mov	w8, #1                          ; =0x1
	str	x8, [x0]
	strb	w8, [x1]
	ldr	x19, [x0]
	cmp	x19, #1
	cset	w0, eq
	mov	w1, #0                          ; =0x0
	bl	_meow_assert
	str	x19, [sp]
Lloh4:
	adrp	x0, l_.str.2@PAGE
Lloh5:
	add	x0, x0, l_.str.2@PAGEOFF
	bl	_printf
	ldp	x29, x30, [sp, #32]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #48
	ret
	.loh AdrpAdd	Lloh4, Lloh5
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:                                ; %entry_block9v1
	sub	sp, sp, #48
	stp	x20, x19, [sp, #16]             ; 16-byte Folded Spill
	stp	x29, x30, [sp, #32]             ; 16-byte Folded Spill
	add	x29, sp, #32
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
Lloh6:
	adrp	x0, l_.str.3@PAGE
Lloh7:
	add	x0, x0, l_.str.3@PAGEOFF
	bl	_puts
	mov	w19, #1                         ; =0x1
	str	x19, [sp, #8]
	add	x8, sp, #8
	add	x0, sp, #8
	orr	x1, x8, #0x1
	bl	__Z6demo_1PyPh
Lloh8:
	adrp	x0, l_.str.4@PAGE
Lloh9:
	add	x0, x0, l_.str.4@PAGEOFF
	bl	_puts
	str	x19, [sp]
	mov	x8, sp
	mov	x0, sp
	orr	x1, x8, #0x1
	bl	__Z6demo_2PyPh
	mov	w0, #0                          ; =0x0
	ldp	x29, x30, [sp, #32]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #48
	ret
	.loh AdrpAdd	Lloh8, Lloh9
	.loh AdrpAdd	Lloh6, Lloh7
	.cfi_endproc
                                        ; -- End function
	.section	__TEXT,__const
l_.str:                                 ; @.str
	.asciz	"bad analysis at location %u\n"

l_.str.1:                               ; @.str.1
	.asciz	"I'm allegedly unreachable!"

l_.str.2:                               ; @.str.2
	.asciz	"*foo = 0x%llx\n"

l_.str.3:                               ; @.str.3
	.asciz	"demo 1"

l_.str.4:                               ; @.str.4
	.asciz	"demo 2"

.subsections_via_symbols
