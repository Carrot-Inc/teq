package scala.deriving

// Scala's derivation protocol. The compiler synthesizes a given `Mirror.Of[T]` for a case
// class, a case object, an enum or a sealed hierarchy (src/typer/derive.rs): a value of
// scala.runtime's mirror classes, typed as `Mirror.Product`/`Mirror.Sum` refined with the
// members below, exactly as scalac types the mirror it synthesizes.

sealed trait Mirror:
  /** The mirrored type without its type arguments applied. */
  type MirroredMonoType
  /** The mirrored type as it was asked for. */
  type MirroredType
  /** The name of the type, as a literal type. */
  type MirroredLabel <: String
  /** The types of the fields of a product, of the cases of a sum, as a tuple. */
  type MirroredElemTypes <: Tuple
  /** Their names, as a tuple of literal types. */
  type MirroredElemLabels <: Tuple

object Mirror:
  trait Sum extends Mirror:
    def ordinal(x: MirroredMonoType): Int

  trait Product extends Mirror:
    def fromProduct(p: scala.Product): MirroredMonoType
    def fromTuple(t: MirroredElemTypes): MirroredMonoType = fromProduct(t)
    def fromProductTyped[P <: scala.Product](x: P): MirroredMonoType = fromProduct(x)

  // scalac's `Singleton` fixes `MirroredMonoType = this.type`, since a case object is its own
  // mirror there; here the mirror is an object of its own, and the compiler's refinement says
  // what it mirrors.
  trait Singleton extends Product:
    type MirroredElemTypes = EmptyTuple
    type MirroredElemLabels = EmptyTuple

  type Of[T] = Mirror { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }
  type ProductOf[T] = Mirror.Product { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }
  type SumOf[T] = Mirror.Sum { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }
