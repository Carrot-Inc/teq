package scala.compiletime

// The members of scala.compiletime are evaluated by the compiler where an inline method
// expands (src/typer/inline.rs); these bodies stand for their signatures.

inline def erasedValue[T]: T = ???

inline def constValue[T]: T = ???

inline def constValueOpt[T]: Option[T] = ???

inline def constValueTuple[T <: Tuple]: T = ???

inline def summonInline[T]: T = ???

transparent inline def summonFrom[T](f: Nothing => T): T = ???

inline def summonAll[T <: Tuple]: T = ???

inline def error(inline msg: String): Nothing = ???

inline def requireConst(inline x: Any): Unit = ???

transparent inline def codeOf(inline arg: Any): String = ???

inline def uninitialized: Nothing = ???

// The right-hand side of a deferred given in a trait (`given x: T = deferred`), which the classes
// implementing the trait implement by a search where they are defined; anywhere else an error.
inline def deferred: Nothing = ???
