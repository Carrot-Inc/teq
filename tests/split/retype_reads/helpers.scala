/** Two package blocks of one file: the macro calls a def of the second. */
package reads.first:
  object G:
    def value: Int = 5

package reads.second:
  object H:
    def value: Int = 1
