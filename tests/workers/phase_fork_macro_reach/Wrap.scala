import Lit.*

object Wrap:
  inline def coded: Show[Model.Code] = new Show[Model.Code]:
    def show(t: Model.Code): String = code"QQ".value + ":" + t.value
