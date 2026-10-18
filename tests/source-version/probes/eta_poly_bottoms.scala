type P = [A] => A => A
type M = Null
type K[X] <: Null = X match { case Int => Null }
type U[X] = X match { case Int => Null }
trait T { type N <: Null; def g(f: P | N) = f _ }
def t1[N <: Null](f: P | N) = f _
def t2[N <: Nothing](f: P | N) = f _
def t3[N <: M](f: P | N) = f _
def t4[N <: Q, Q <: Null](f: N | P) = f _
def t5[X](f: P | K[X]) = f _
def t6[N >: Null <: Null](f: P | N) = f _
def t7[N <: Null](f: P | (N | Null)) = f _
def t8[N <: Null](f: (P & Serializable) | N) = f _
def t9(f: P | M) = f _
def t10(f: P | U[Int]) = f _
def t11[F0 <: P, F1 <: F0, F2 <: F1, F3 <: F2, F4 <: F3, F5 <: F4, F6 <: F5, F7 <: F6, F8 <: F7, F9 <: F8, F10 <: F9, F11 <: F10, F12 <: F11, F13 <: F12, F14 <: F13, F15 <: F14, F16 <: F15, F17 <: F16, F18 <: F17, F19 <: F18, F20 <: F19, F21 <: F20, F22 <: F21, F23 <: F22, F24 <: F23, F25 <: F24, F26 <: F25, F27 <: F26, F28 <: F27, F29 <: F28, F30 <: F29, F31 <: F30, F32 <: F31, F33 <: F32, F34 <: F33, F35 <: F34, F36 <: F35, F37 <: F36, F38 <: F37, F39 <: F38, F40 <: F39](f: F40) = f _
val n1: P & Serializable = null
val n2 = n1 _
val n3: P | ([A] => List[A] => A) = null
val n4 = n3 _
