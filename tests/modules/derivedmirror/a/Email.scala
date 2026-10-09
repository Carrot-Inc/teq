package dma2

import scala.deriving.Mirror

// The mirror a `derives` clause and a summon synthesize for a case class, which scalac's
// `Synthesizer` writes as the companion cast to the mirror's refinement: written so in the pickle
// of `Email.derived$Schema` and of `Library.mirror`, no body withheld, and read back downstream
// as the mirror the typer synthesizes there.
trait Schema[A]:
  def make(p: Product): A

object Schema:
  def derived[A](using m: Mirror.ProductOf[A]): Schema[A] = new Schema[A]:
    def make(p: Product): A = m.fromProduct(p)

case class Email(value: String) derives Schema
case class Point(x: Int, y: Int) derives Schema

object Library:
  def mirror: Mirror.ProductOf[Point] = summon[Mirror.ProductOf[Point]]
