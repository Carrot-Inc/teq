package bl

class Head:
  def x: Int = 1

// A package block's classes belong to the file that holds it.
package inner:
  class Nested:
    def y: Int = 2
  object Tools:
    def z: String = "z"
