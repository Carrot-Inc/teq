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

  // A case object and an enum's singleton case are their own mirrors, as scala-library's.
  trait Singleton extends Product:
    type MirroredMonoType = this.type
    type MirroredType = this.type
    type MirroredElemTypes = EmptyTuple
    type MirroredElemLabels = EmptyTuple
    def fromProduct(p: scala.Product): MirroredMonoType = this

  /** A proxy for Scala 2 singletons, which do not inherit `Singleton` directly. */
  class SingletonProxy(val value: AnyRef) extends Product:
    type MirroredMonoType = value.type
    type MirroredType = value.type
    type MirroredElemTypes = EmptyTuple
    type MirroredElemLabels = EmptyTuple
    def fromProduct(p: scala.Product): MirroredMonoType = value

  type Of[T] = Mirror { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }
  type ProductOf[T] = Mirror.Product { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }
  type SumOf[T] = Mirror.Sum { type MirroredType = T; type MirroredMonoType = T; type MirroredElemTypes <: Tuple }

  // scala-library's extensions of a product's mirror: over a product whose own mirror gives
  // elements of the receiver's element types, and over the tuple of those.
  extension [T](p: ProductOf[T])
    def fromProductTyped[A <: scala.Product, Elems <: p.MirroredElemTypes](a: A)(using ProductOf[A] { type MirroredElemTypes = Elems }): T =
      p.fromProduct(a)

    def fromTuple(t: p.MirroredElemTypes): T =
      p.fromProduct(t)
