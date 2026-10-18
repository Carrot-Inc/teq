//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// jars: tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm
// tapir 1.13.29's endpoint inputs joined element by element (`ParamConcat`, `Join[P, S]` folded through
// `Join.Fold.step` and `TupleFoldInstances.tN`), each `.in(...)` resolved over the input tuple of the endpoint
// before it, the variable the settlement leaves open as given-open-variable's rule has it
// (`Inferencing.interpolateTypeVars` leaves the nonvariant occurrence to the expected type) and bounded by that
// tuple on both sides: in an inferred val, a generic function's argument, a chain ending in `.mapIn` or
// `.serverLogic` and a lambda's result. Every endpoint's type is the exact one scalac infers (`Endpoint` is
// invariant in its input).
import sttp.tapir.*
import sttp.tapir.server.ServerEndpoint
import sttp.monad.IdentityMonad
import sttp.shared.Identity

final case class Id(value: String) extends AnyVal

object Endpoints:
  type In = (Id, Int, Option[Int], Option[String])
  type Four = Endpoint[Unit, In, Unit, String, Any]
  def keep[A](a: A): A = a
  def eval[A](f: Int => A): A = f(0)
  val inferred = endpoint.in(path[String]("id").map(Id.apply)(_.value)).in(query[Int]("q1")).in(query[Option[Int]]("q2")).in(query[Option[String]]("q3")).out(stringBody)
  val kept = keep(endpoint.in(path[String]("id").map(Id.apply)(_.value)).in(query[Int]("q1")).in(query[Option[Int]]("q2")).in(query[Option[String]]("q3")).out(stringBody))
  val mapped = endpoint.in(path[String]("id").map(Id.apply)(_.value)).in(query[Int]("q1")).in(query[Option[Int]]("q2")).in(query[Option[String]]("q3")).out(stringBody).mapIn(x => x)(x => x)
  val evaluated = eval(_ => endpoint.in(path[String]("id").map(Id.apply)(_.value)).in(query[Int]("q1")).in(query[Option[Int]]("q2")).in(query[Option[String]]("q3")).out(stringBody))
  val served = endpoint.in(path[String]("id").map(Id.apply)(_.value)).in(query[Int]("q1")).in(query[Option[Int]]("q2")).in(query[Option[String]]("q3")).out(stringBody)
    .serverLogic[Identity] { case (id, n, a, b) => Right(s"${id.value}:$n:${a.getOrElse(0)}:${b.getOrElse("-")}") }
  val checked: Four = inferred
  val all: List[Four] = List(inferred, kept, mapped, evaluated)
  val full: ServerEndpoint.Full[Unit, Unit, In, Unit, String, Any, Identity] = served

@main def run(): Unit =
  Endpoints.all.foreach(e => println(e.show))
  println(Endpoints.full.show)
  println(Endpoints.served.logic(IdentityMonad)(())((Id("a"), 1, Some(2), None)))
  println(Endpoints.served.logic(IdentityMonad)(())((Id("b"), 3, None, Some("c"))))
