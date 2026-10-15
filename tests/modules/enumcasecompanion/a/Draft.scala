package eca

import scala.deriving.Mirror

// An enum's class cases, whose companions `Desugar.classDef` gives as a case class's: the
// cases' `Mirror.Product` (`useCompanionAsProductMirror`), read by the downstream from the
// pickle, of a package's enum and of a class's.
enum Draft:
  case Store(title: Option[String])
  case Email(subject: Option[String], on: Boolean = false)
  case Empty

class Outer:
  enum In:
    case A(x: Int)
    case B

object Here:
  val email: Mirror.ProductOf[Draft.Email] = summon[Mirror.ProductOf[Draft.Email]]
