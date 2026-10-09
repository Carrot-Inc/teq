package tx

// The type parameters of an extension method's own clause, after the receiver's.
extension (x: Int) def f[A <: java.lang.Runnable](a: A): A = a
