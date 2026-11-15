package inc

class Api(val base: Int):
  class Request(val k: Int):
    def sum: Int = base + k
  class Two(val k: Int)(val j: Int):
    def sum: Int = base + k + j
  class Defaults(val k: Int = 9, val j: Int = 1):
    def sum: Int = base + k + j
  class Over(val k: Int, val j: Int):
    def this(k: Int) = this(k, 2)
    def sum: Int = base + k + j
  class Node(val up: Api, val label: String):
    def this(label: String) = this(Api.this, label)
    def path: String = base.toString + "/" + up.base + "/" + label

object Stock:
  val api: Api = new Api(10)
