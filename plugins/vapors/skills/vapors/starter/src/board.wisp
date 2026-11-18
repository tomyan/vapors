<script>
use crate::game::{Tile, World};

let world: Signal<World> = Signal::new(World::new());
let rows: Memo<Vec<Vec<Tile>>> = Memo::new(move || world.get().tiles());
let crossings: Memo<u32> = Memo::new(move || world.get().crossings);
let hits: Memo<u32> = Memo::new(move || world.get().hits);
</script>

<style>
:global(body) { margin: 0; background-color: #171b1f; color: #e4e8ec; font-size: 16px; }
.board { display: flex; flex-direction: column; align-items: center; padding: 16px; }
.row { display: flex; }
.tile { width: 40px; height: 40px; margin: 1px; background-color: #3c4248; }
.tile.grass { background-color: #2f6b3a; }
.tile.car { background-color: #d5533a; }
.tile.frog { background-color: #8bd450; }
</style>

<div class="board">
  <p>Arrow keys move. Crossings {crossings} · Hits {hits}</p>
  {for row in rows.get()}
    <div class="row">
      {for tile in row.get()}
        <div class="tile" class:grass={tile.get() == Tile::Grass} class:car={tile.get() == Tile::Car} class:frog={tile.get() == Tile::Frog}></div>
      {/for}
    </div>
  {/for}
</div>
