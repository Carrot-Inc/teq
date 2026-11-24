// A protected constant read through a val selected on a value, from a class outside its access boundary: scalac
// folds the read to its literal (constToLiteral), so no protected accessor is needed and the program runs.
package base
class Base:
  protected final val k = 3
  protected final val m: 4 = 4
