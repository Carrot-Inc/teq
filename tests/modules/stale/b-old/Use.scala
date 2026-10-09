package stb

import sta.*

class Cache(store: Store):
  def lookup(key: String, default: Int): String = store.get(key).fold("none")(_.toString)
  def stale: Boolean = true

object Use:
  def main(args: Array[String]): Unit =
    println(new Cache(new Store).lookup("abc", 1))
