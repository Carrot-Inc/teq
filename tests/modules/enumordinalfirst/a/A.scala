// An upstream's enums, of value cases alone and of value and class cases mixed, for the client that completes each
// enum before its companion (b/B.scala).
package eofa

enum Color:
  case Dark, Light

enum Mixed:
  case A
  case B(x: Int)
  case C
