package dia

trait Animal:
  def name: String
  def sound: String = "..."
case class Dog(name: String) extends Animal:
  override def sound: String = "woof"
