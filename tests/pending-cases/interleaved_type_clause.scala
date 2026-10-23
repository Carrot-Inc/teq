// A method's type parameter clause after a term clause (clause interleaving, standard from 3.6): scalac 3.8.4 accepts
// `def f[A](a: A)(b: Int)[B](c: B)`, teq reports "expected '=', found '['" at the second clause.
def f[A](a: A)(b: Int)[B](c: B): A = a
@main def run(): Unit = println(f(1)(2)("x"))
