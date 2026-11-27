package infer
final abstract class Token[T]
object Token { implicit def materialize: Token["x"] = null.asInstanceOf[Token["x"]] }
class Base[T](implicit val token: Token[T])
class K extends Base
object O extends Base
