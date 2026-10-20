// lila: app/controllers/LilaController.scala:18 (`extends ... with lila.web.ResponseBuilder(using env.executor)
// with http.ResponseBuilder(using env.executor) ...`, where http.ResponseBuilder extends lila.web.ResponseBuilder)
// Two parameterized traits with the same simple name in different packages each get their own
// explicit using argument; neither is an indirectly extended trait needing a synthesized argument.
import scala.concurrent.{ ExecutionContext, ExecutionContextExecutor as Executor }

package web:
  trait ResponseBuilder(using ec: Executor):
    def webEc = ec

package http:
  trait ResponseBuilder(using ec: Executor) extends web.ResponseBuilder:
    def httpEc = ec

class Env(val executor: Executor)

abstract class LilaController(val env: Env)
    extends web.ResponseBuilder(using env.executor)
    with http.ResponseBuilder(using env.executor)

class Main(env: Env) extends LilaController(env)

@main def run() =
  val m = Main(Env(ExecutionContext.global))
  println(m.webEc eq m.httpEc)
