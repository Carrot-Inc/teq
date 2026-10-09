// jars: scala-library tapir-core sttp-model sttp-shared-core sttp-shared-ws magnolia123 scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// tapir 1.13.29's endpoint description language from its jar: `endpoint.get.in(...)`, path
// captures, query and header inputs, bodies, error outputs, `securityIn`, `oneOf` variants with
// status codes, the input and output combinators through `ParamConcat`, `tag`, `description`,
// `Endpoint.show`, `showDetail`, `showPathTemplate` and `PublicEndpoint`.
import sttp.tapir.*
import sttp.model.StatusCode
import sttp.tapir.internal.{RichEndpointInput, RichEndpointOutput}

case class Pet(id: Int, name: String, tags: List[String])
case class ApiError(code: Int, message: String)
sealed trait Failure
case class NotFound(what: String) extends Failure
case class Unauthorized(reason: String) extends Failure

object Api:
  val getPet: PublicEndpoint[(Int, String), String, String, Any] =
    endpoint.get.in("pets" / path[Int]("id")).in(query[String]("q")).out(stringBody).errorOut(stringBody)

  val listPets = endpoint.get
    .in("pets")
    .in(query[Option[Int]]("limit").description("how many"))
    .in(header[Option[String]]("X-Trace"))
    .out(stringBody)
    .out(header[String]("X-Count"))
    .tag("pets")
    .description("all the pets")
    .name("listPets")
    .summary("List pets")

  val createPet = endpoint.post
    .in("pets")
    .in(stringBody.description("the pet"))
    .out(statusCode(StatusCode.Created))
    .out(plainBody[Int])

  val secured: Endpoint[String, Unit, Unit, String, Any] =
    endpoint.securityIn(auth.bearer[String]()).in("me").out(stringBody)

  val matrix = endpoint.get.in("a" / path[Int]("x") / "b" / path[String]("y") / path[Long]("z"))

  val errors = endpoint.get
    .in("things" / path[Int]("id"))
    .out(stringBody)
    .errorOut(
      oneOf[Failure](
        oneOfVariant(StatusCode.NotFound, plainBody[String].map(NotFound(_))(_.what)),
        oneOfVariant(StatusCode.Unauthorized, plainBody[String].map(Unauthorized(_))(_.reason))
      )
    )

  val withStatus = endpoint.get.in("status").out(statusCode.and(stringBody))

  val unitInputs = endpoint.get.in("only" / "fixed").in(emptyInput).out(emptyOutput)

  val mapped = endpoint.get
    .in(("pets" / path[Int]("id") / path[String]("name")).mapTo[(Int, String)])
    .in(query[List[String]]("tag"))

  val paramsSplit = endpoint.get.in(path[Int]("a")).in(path[Int]("b")).in(path[Int]("c"))

object Main:
  def main(args: Array[String]): Unit =
    println(Api.getPet.show)
    println(Api.getPet.showDetail)
    println(Api.getPet.showPathTemplate())
    println(Api.listPets.show)
    println(Api.listPets.info.tags.toString + " " + Api.listPets.info.description + " " + Api.listPets.info.name + " " + Api.listPets.info.summary)
    println(Api.createPet.show)
    println(Api.createPet.method.toString)
    println(Api.secured.show)
    println(Api.secured.showDetail)
    println(Api.matrix.show)
    println(Api.matrix.showPathTemplate())
    println(Api.errors.show)
    println(Api.errors.showDetail)
    println(Api.withStatus.show)
    println(Api.unitInputs.show)
    println(Api.mapped.show)
    println(Api.paramsSplit.show)
    println(Api.paramsSplit.input.show)
    val in: EndpointInput[(Int, String)] = path[Int]("i").and(query[String]("s"))
    val out: EndpointOutput[(String, Int)] = stringBody.and(header[Int]("n"))
    println(in.show + " | " + out.show)
    val basic = endpoint.in(in).out(out)
    println(basic.show)
    println(Api.getPet.input.show)
    println(Api.getPet.errorOutput.show + " " + Api.getPet.output.show)
    println(Api.listPets.input.asVectorOfBasicInputs().map(_.show).mkString(", "))
    println(Api.errors.errorOutput.asBasicOutputsList.map(_.map(_.show)).toString)
    println(infallibleEndpoint.show)
    println(Api.getPet.info.attributes.isEmpty.toString + " " + Api.listPets.info.deprecated)
    println(Api.getPet.mapIn { case (i, s) => Pet(i, s, Nil) } (p => (p.id, p.name)).show)
    println(Api.getPet.prependIn(query[Long]("first")).show)
    println(Api.getPet.mapErrorOut(ApiError(500, _))(_.message).errorOutput.show)
