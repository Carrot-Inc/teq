package stb

import sta.*

class Cache(store: Store):
  def lookup(key: String): Int = store.get(key).getOrElse(0)

object Use:
  def main(args: Array[String]): Unit =
    println(new Cache(new Store).lookup("abc"))
