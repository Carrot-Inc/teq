package packageoverloads {
  trait Operations {
    def choose(value: Int): Int = value + 1
    def choose(value: String): Int = value.length
  }
}
package object packageoverloads extends packageoverloads.Operations
