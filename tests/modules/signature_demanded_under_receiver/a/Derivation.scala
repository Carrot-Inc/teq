package derivation

trait Show[T]
object Show
trait Eq[T]
object Eq
trait Order[T]
object Order

class ShowOf[A] extends Show[A]
class EqOf[A] extends Eq[A]
class OrderOf[A] extends Order[A]

extension (x: Show.type) def derived[A]: Show[A] = new ShowOf[A]
extension (x: Eq.type) def derived[A]: Eq[A] = new EqOf[A]
extension (x: Order.type) def derived[A]: Order[A] = new OrderOf[A]
