package cda

// A case class of an object with a derived codec and a companion val, constructed downstream: the
// companion is initialised before the construction, as scalac's `apply` initialises it, in the
// whole build and over this module's products alike (the manifest lists it among its inits).
import scala.deriving.Mirror

trait Codec[A]:
  def name: String
object Codec:
  inline def derived[A](using m: Mirror.ProductOf[A]): Codec[A] = new Codec[A]:
    def name: String = compiletime.constValue[m.MirroredLabel]

object FlagModels:
  final case class Flags(on: Boolean, names: List[String]) derives Codec
  object Flags:
    println("Flags initialised")
    val empty: Flags = Flags(false, Nil)
