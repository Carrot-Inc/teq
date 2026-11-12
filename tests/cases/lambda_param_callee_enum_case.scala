// A lambda passed to a generic method whose type parameter nothing else fixes, its body an enum case applied
// to the parameter: the parameter's type is the case's parameter's, as for a case class.
enum Node:
  case Wrapped(inner: Inner)
  case Plain

enum Inner:
  case Leaf(tag: String)
  case Empty

final class Sink[A](val handle: PartialFunction[A, Unit]):
  def contramap[B](f: B => A): Sink[B] = new Sink[B]({ case b if handle.isDefinedAt(f(b)) => handle(f(b)) })

def sinkOf(pf: PartialFunction[Node, Unit]): Sink[Node] = new Sink(pf)

import Node.*

@main def run(): Unit =
  var seen = ""
  val sink = sinkOf {
    case Wrapped(Inner.Leaf(tag)) => seen = tag
  }.contramap(Wrapped(_))
  sink.handle(Inner.Leaf("leaf"))
  println(seen)
