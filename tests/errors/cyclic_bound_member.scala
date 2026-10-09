// expect: 9:26: error: value foo is not a member of A
// expect: 10:31: error: value foo is not a member of A
// expect: 11:43: error: value foo is not a member of A
// A member looked up through a type parameter whose bound leads back to it is not found, where
// the lookup recursed until the stack overflowed; scalac reports an illegal cyclic reference
// (scala3's neg/i20317a meets the first shape once its alias parses: `type S[A] =` with the
// type on the next line).
type S[A] = A
def f[A <: S[A]](i: A) = i.foo
def g[A <: B, B <: A](i: A) = i.foo
def h[A <: B & C, B <: A, C <: A](i: A) = i.foo
