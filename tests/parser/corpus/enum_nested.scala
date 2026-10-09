package app.util

import app.util.Storage.Key
import app.util.Storage.Key.*
import app.util.Storage.Layers.Layer

object Storage:
  final case class RawString()

  enum Key[T](val id: String):
    case Token extends Key[RawString]("token")
    case DeviceId extends Key[Int]("device_id")
    case WideMenu extends Key[Boolean]("wide_menu")
    case Seen extends Key[List[Int]]("seen")

    def storageName: String = Key.prefix + id

  object Key:
    val prefix: String = "acme_"
    def byId(id: String): Option[Key[?]] = values.find(_.id == id)

  private val store = scala.collection.mutable.Map[String, Any]()

  def set[T](key: Key[T], value: T): Unit = store(key.storageName) = value
  def get[T](key: Key[T]): Option[T] = store.get(key.storageName).map(_.asInstanceOf[T])
  def getRaw(key: Key[RawString]): String = key.storageName

  object Layers:
    enum Layer(val z: Int):
      case Base extends Layer(0)
      case Modal extends Layer(100)
      case Toast(offset: Int) extends Layer(200 + offset)

      def above(other: Layer): Boolean = z > other.z

class Registry:
  val defaults: Map[Key[?], String] = Map(DeviceId -> "0", WideMenu -> "false")
  def describe(k: Key[?]): String = defaults.getOrElse(k, "<none>")

@main def main(): Unit =
  Storage.set(DeviceId, 42)
  Storage.set(WideMenu, true)
  Storage.set(Key.Seen, List(1, 2, 3))
  val device: Option[Int] = Storage.get(DeviceId)
  println(device.map(_ + 1))
  println(Storage.get(WideMenu).map(!_))
  println(Storage.get(Seen).map(_.sum))
  println(Storage.get(Token))
  println(Storage.getRaw(Token))
  println(Key.byId("seen"))
  println(Key.byId("nope"))
  println(Key.values.map(_.storageName).toList)
  val registry = Registry()
  println(Key.values.toList.map(registry.describe))
  println(Layer.Toast(5).above(Layer.Modal))
  println(Layer.Base.above(Layer.Modal))
  println(List(Layer.Modal, Layer.Toast(1), Layer.Base).sortBy(_.z))
  println(Layer.Toast(1).z)
