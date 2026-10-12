// expect: 15:67: error: value nope is not a member of String
// expect: 1 error found
// absent: 16:
// absent: 18:
// absent: 19:
// An export through a val whose initializer holds an error has a path of the error type, of which
// every named member is an error symbol (dotty's `findMember`, `newErrorSymbol`): its forwarders are
// of the error type, and neither the clause nor their uses report anything past the first error.
class Cache[K, V]:
  def async(k: K): Option[V] = None
  def invalidate(k: K): Unit = ()
class Api:
  def sync[K, V](compute: K => V, default: K => V): Cache[K, V] = Cache()
final class TeamCached(api: Api):
  private val teamIdsCache = api.sync[String, Int](compute = u => u.nope, default = _ => 0)
  export teamIdsCache.{ async as teamIds, invalidate as invalidateTeamIds }
def use(t: TeamCached): Unit =
  t.invalidateTeamIds("x")
  println(t.teamIds("y"))
