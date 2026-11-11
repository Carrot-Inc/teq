// An upstream's enums, of value cases alone and of value and class cases mixed, for the client that reaches each
// companion first (b/B.scala).
package ecfa

enum Color:
  case Dark, Light

enum Mixed:
  case A
  case B(x: Int)
  case C
