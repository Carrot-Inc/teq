package scala.compiletime.ops

// Type-level operations on literal types, evaluated by the compiler where both operands are
// literal types (src/typer/inline.rs); these declarations give them a name and a kind.

object int:
  opaque type +[X <: Int, Y <: Int] <: Int = Int
  opaque type -[X <: Int, Y <: Int] <: Int = Int
  opaque type *[X <: Int, Y <: Int] <: Int = Int
  opaque type /[X <: Int, Y <: Int] <: Int = Int
  opaque type %[X <: Int, Y <: Int] <: Int = Int
  opaque type <<[X <: Int, Y <: Int] <: Int = Int
  opaque type >>[X <: Int, Y <: Int] <: Int = Int
  opaque type >>>[X <: Int, Y <: Int] <: Int = Int
  opaque type ^[X <: Int, Y <: Int] <: Int = Int
  opaque type BitwiseAnd[X <: Int, Y <: Int] <: Int = Int
  opaque type BitwiseOr[X <: Int, Y <: Int] <: Int = Int
  opaque type Min[X <: Int, Y <: Int] <: Int = Int
  opaque type Max[X <: Int, Y <: Int] <: Int = Int
  opaque type Abs[X <: Int] <: Int = Int
  opaque type Negate[X <: Int] <: Int = Int
  opaque type S[N <: Int] <: Int = Int
  opaque type ToString[X <: Int] <: String = String
  opaque type <[X <: Int, Y <: Int] <: Boolean = Boolean
  opaque type >[X <: Int, Y <: Int] <: Boolean = Boolean
  opaque type <=[X <: Int, Y <: Int] <: Boolean = Boolean
  opaque type >=[X <: Int, Y <: Int] <: Boolean = Boolean

object string:
  opaque type +[X <: String, Y <: String] <: String = String
  opaque type Length[X <: String] <: Int = Int

object boolean:
  opaque type &&[X <: Boolean, Y <: Boolean] <: Boolean = Boolean
  opaque type ||[X <: Boolean, Y <: Boolean] <: Boolean = Boolean
  opaque type ^[X <: Boolean, Y <: Boolean] <: Boolean = Boolean
  opaque type ![X <: Boolean] <: Boolean = Boolean

object any:
  opaque type ==[X, Y] <: Boolean = Boolean
  opaque type !=[X, Y] <: Boolean = Boolean
  opaque type ToString[X] <: String = String
  opaque type IsConst[X] <: Boolean = Boolean
