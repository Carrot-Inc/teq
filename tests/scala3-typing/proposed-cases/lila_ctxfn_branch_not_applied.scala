// lila's dsl.byIdProj is a polymorphic function value whose result is a context function
// (`[I] => (I, Bdoc) => BSONWriter[I] ?=> Fu[Option[D]]`); it is called in a case of a
// pattern-matching lambda whose result type is still a type variable.
trait Writer[A]:
  def write(a: A): String
given Writer[Int] = (a: Int) => s"#$a"

def byIdProj: [I] => (I, String) => Writer[I] ?=> Option[String] =
  [I] => (id: I, proj: String) => Some(summon[Writer[I]].write(id) + " with " + proj)
def one(id: Any): Option[String] = Some(s"one $id")

def app[A, B](a: A)(f: A => B): B = f(a)

// dsl.scala:318 -- a case body, expected type B undetermined
def byId(id: Any): Option[String] = app(id):
  case (i: Int, proj: String) => byIdProj(i, proj)
  case other => one(other)

// ReportApi.scala:676 / AuthorizationRequest.scala:70 -- branches with no expected type
def pick(flag: Boolean) =
  if flag then byIdProj(1, "p") else one(2)

// Challenge.scala:258 -- a lambda body against G[B], G undetermined (cats traverse over a
// method whose result is a context function, like OAuthServer.auth: FuRaise[..])
def auth(i: Int): Writer[Int] ?=> Option[String] = Some(summon[Writer[Int]].write(i))
trait Applicative[G[_]]:
  def name: String
given Applicative[Option] with
  def name = "Option"
def traverseName[G[_], B](f: Int => G[B])(using ap: Applicative[G]): String = ap.name

@main def run() =
  println(byId((7, "name")))
  println(byId("x"))
  println(pick(true).map(_.length))
  println(traverseName(i => auth(i)))
