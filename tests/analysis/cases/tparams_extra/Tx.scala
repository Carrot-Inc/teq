package tx

// The type parameters of an extension method's own clause, after the receiver's.
extension (x: Int) def f[A <: String](a: A): A = a

class Extra(val s: String)
