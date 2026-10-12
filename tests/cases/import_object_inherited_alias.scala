// A class header importing an object's inherited type aliases is completed before the object
// (it stands above it): the object is completed when its members are looked up, as dotty
// completes a symbol on demand.
package user:
  import db.dsl.{ *, given }
  final class Repo(val c: Coll, d: Bdoc, r: ReadPref):
    def name: Coll = c + r(ReadPref) + d.sum

  @main def run(): Unit =
    println(Repo(db.dsl.coll("x"), List(1, 2), _.pri).name)

package db:
  trait Handlers:
    type Bdoc = List[Int]
  trait dsl:
    type ReadPref = ReadPref.type => Int
    object ReadPref:
      val pri: Int = 7
    type Coll = String
    def coll(n: String): Coll = n
  object dsl extends dsl with Handlers
