// Imports through paths of vals: a class parameter's val, a val of it and an object nested in it,
// and a parameter's field (dotty types the qualifier as a path, `Namer`'s import completer).
class Api:
  val userApi = 1
  val gameApi = 2
  object Nested:
    val deep = 30
class Env:
  val api = new Api
class Ctrl(val env: Env):
  import env.api.{userApi, gameApi}
  import env.api.Nested.*
  def show = userApi + gameApi + deep
case class G(id: Int)
case class W(game: G)
def f(init: W) =
  import init.game.id
  id
@main def main() = println((new Ctrl(new Env).show, f(W(G(3)))))
