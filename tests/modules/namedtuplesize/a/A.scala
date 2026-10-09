package nta

import scala.NamedTuple.NamedTuple

// A body selecting a named tuple's `size`, which scala-library exports from
// `NamedTupleDecomposition`: the products carry it as that member.
def count[N <: Tuple, V <: Tuple](x: NamedTuple[N, V]): Tuple.Size[V] = x.size
