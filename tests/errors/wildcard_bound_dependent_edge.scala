// A realistic shape of the dependent bound over a wildcard: `Graph[?, IntEdge]` with `E <: Edge[N]` is checked
// against `Edge[Graph[?, IntEdge]#N]` (Checking.scala 118, Types.scala 2794), and `Edge` is invariant, so scalac
// 3.8.4 rejects the parameter type (E057):
//   9:21 Type argument IntEdge does not conform to upper bound Edge[Graph[?, IntEdge]#N]
// teq is to reject it as scalac does; substituting the wildcard accepts it (`IntEdge <: Edge[?]`).
trait Edge[N]
class IntEdge extends Edge[Int]
class Graph[N, E <: Edge[N]]
def size(g: Graph[?, IntEdge]): Int = 0
// expect: 9:22: error: type argument IntEdge does not conform to upper bound Edge[Graph[?, IntEdge]#N]
// expect: 1 error found
