package ita

// Types declared `infix`, whose pickles say so (`TreePickler`'s `INFIX`): a downstream module writes
// them infix without the alphanumeric-infix warning (`SymUtils.isDeclaredInfix`).
infix type or[A, B] = Either[A, B]
infix class Both[A, B](val a: A, val b: B)
