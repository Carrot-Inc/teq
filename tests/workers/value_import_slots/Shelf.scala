// A shelf's row is an object of the instance: an import of its members goes through the value the shelf is held in
// (`import sh.row.*`), an entry of the table of value imports every worker shares. Eight files of eight methods
// each import through a value of their own: two workers typing them push to the table at once.
package shelves

class Shelf(val mark: String):
  object row:
    val label = mark
    def doubled: String = mark + mark
