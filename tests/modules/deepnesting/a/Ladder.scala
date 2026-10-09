// Thirty branches and thirty nested blocks: the writer asked each for the type it writes once per enclosing one,
// 2^depth, before `natural_type` kept its answers.
package dna

object Ladder:
  def rank(x: Int): Int =
    if x == 0 then 0
    else if x == 1 then 1
    else if x == 2 then 2
    else if x == 3 then 3
    else if x == 4 then 4
    else if x == 5 then 5
    else if x == 6 then 6
    else if x == 7 then 7
    else if x == 8 then 8
    else if x == 9 then 9
    else if x == 10 then 10
    else if x == 11 then 11
    else if x == 12 then 12
    else if x == 13 then 13
    else if x == 14 then 14
    else if x == 15 then 15
    else if x == 16 then 16
    else if x == 17 then 17
    else if x == 18 then 18
    else if x == 19 then 19
    else if x == 20 then 20
    else if x == 21 then 21
    else if x == 22 then 22
    else if x == 23 then 23
    else if x == 24 then 24
    else if x == 25 then 25
    else if x == 26 then 26
    else if x == 27 then 27
    else if x == 28 then 28
    else if x == 29 then 29
    else 30

  def nested(x: Int): Int =
    { val y29 = { val y28 = { val y27 = { val y26 = { val y25 = { val y24 = { val y23 = { val y22 = { val y21 = { val y20 = { val y19 = { val y18 = { val y17 = { val y16 = { val y15 = { val y14 = { val y13 = { val y12 = { val y11 = { val y10 = { val y9 = { val y8 = { val y7 = { val y6 = { val y5 = { val y4 = { val y3 = { val y2 = { val y1 = { val y0 = x; y0 + 1 }; y1 + 1 }; y2 + 1 }; y3 + 1 }; y4 + 1 }; y5 + 1 }; y6 + 1 }; y7 + 1 }; y8 + 1 }; y9 + 1 }; y10 + 1 }; y11 + 1 }; y12 + 1 }; y13 + 1 }; y14 + 1 }; y15 + 1 }; y16 + 1 }; y17 + 1 }; y18 + 1 }; y19 + 1 }; y20 + 1 }; y21 + 1 }; y22 + 1 }; y23 + 1 }; y24 + 1 }; y25 + 1 }; y26 + 1 }; y27 + 1 }; y28 + 1 }; y29 + 1 }
