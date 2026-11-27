package a2
trait JsAny extends AnyRef
class JsObject extends JsAny { def n: Int = 31 }
trait HTMLAttributes[T] extends JsObject
trait Component[P] extends JsObject
trait IPersonaSharedProps extends HTMLAttributes[PersonaCoinBase]
trait PersonaCoinBase extends Component[IPersonaCoinProps]
trait IPersonaCoinProps extends IPersonaSharedProps
class Coin extends IPersonaCoinProps
