<script>
use crate::dashboard::CHART_BARS;
#[prop] let series: Signal<usize> = Signal::new(0);
#[prop] let compare: Signal<bool> = Signal::new(false);
let slots: Vec<usize> = (0..CHART_BARS).collect();
</script>

<style>
/* The bars' heights are rules by series and slot (`:nth-child`), since a style cannot take a
   computed height: see FLAGS.md. They are the spec's `bar_height` and `previous_height`, written
   out; the layout test holds every bar to the spec's functions. */
.plot { position: absolute; left: 20px; top: 72px; width: 600px; height: 228px; display: flex; }
.slot { position: relative; flex: none; width: 25px; height: 228px; }
.cur { position: absolute; bottom: 0; left: 4px; width: 17px; border-radius: 4px 4px 0 0; background-color: #6366f1; }
.prev { position: absolute; bottom: 0; left: 3px; width: 8px; border-radius: 4px 4px 0 0; background-color: #c7d2fe; }
.cmp .cur { left: 13px; width: 8px; }
.s0 .slot:nth-child(1) .cur { height: 40px; } .s0 .slot:nth-child(1) .prev { height: 126px; } .s0 .slot:nth-child(2) .cur { height: 99px; } .s0 .slot:nth-child(2) .prev { height: 118px; } .s0 .slot:nth-child(3) .cur { height: 121px; } .s0 .slot:nth-child(3) .prev { height: 102px; }
.s0 .slot:nth-child(4) .cur { height: 139px; } .s0 .slot:nth-child(4) .prev { height: 81px; } .s0 .slot:nth-child(5) .cur { height: 153px; } .s0 .slot:nth-child(5) .prev { height: 60px; } .s0 .slot:nth-child(6) .cur { height: 157px; } .s0 .slot:nth-child(6) .prev { height: 42px; }
.s0 .slot:nth-child(7) .cur { height: 147px; } .s0 .slot:nth-child(7) .prev { height: 66px; } .s0 .slot:nth-child(8) .cur { height: 127px; } .s0 .slot:nth-child(8) .prev { height: 59px; } .s0 .slot:nth-child(9) .cur { height: 101px; } .s0 .slot:nth-child(9) .prev { height: 74px; }
.s0 .slot:nth-child(10) .cur { height: 75px; } .s0 .slot:nth-child(10) .prev { height: 91px; } .s0 .slot:nth-child(11) .cur { height: 53px; } .s0 .slot:nth-child(11) .prev { height: 106px; } .s0 .slot:nth-child(12) .cur { height: 82px; } .s0 .slot:nth-child(12) .prev { height: 117px; }
.s0 .slot:nth-child(13) .cur { height: 74px; } .s0 .slot:nth-child(13) .prev { height: 120px; } .s0 .slot:nth-child(14) .cur { height: 92px; } .s0 .slot:nth-child(14) .prev { height: 112px; } .s0 .slot:nth-child(15) .cur { height: 114px; } .s0 .slot:nth-child(15) .prev { height: 96px; }
.s0 .slot:nth-child(16) .cur { height: 132px; } .s0 .slot:nth-child(16) .prev { height: 75px; } .s0 .slot:nth-child(17) .cur { height: 146px; } .s0 .slot:nth-child(17) .prev { height: 87px; } .s0 .slot:nth-child(18) .cur { height: 150px; } .s0 .slot:nth-child(18) .prev { height: 70px; }
.s0 .slot:nth-child(19) .cur { height: 140px; } .s0 .slot:nth-child(19) .prev { height: 60px; } .s0 .slot:nth-child(20) .cur { height: 120px; } .s0 .slot:nth-child(20) .prev { height: 32px; } .s0 .slot:nth-child(21) .cur { height: 94px; } .s0 .slot:nth-child(21) .prev { height: 79px; }
.s0 .slot:nth-child(22) .cur { height: 109px; } .s0 .slot:nth-child(22) .prev { height: 97px; } .s0 .slot:nth-child(23) .cur { height: 87px; } .s0 .slot:nth-child(23) .prev { height: 111px; } .s0 .slot:nth-child(24) .cur { height: 75px; } .s0 .slot:nth-child(24) .prev { height: 122px; }
.s1 .slot:nth-child(1) .cur { height: 142px; } .s1 .slot:nth-child(1) .prev { height: 116px; } .s1 .slot:nth-child(2) .cur { height: 156px; } .s1 .slot:nth-child(2) .prev { height: 95px; } .s1 .slot:nth-child(3) .cur { height: 160px; } .s1 .slot:nth-child(3) .prev { height: 78px; }
.s1 .slot:nth-child(4) .cur { height: 150px; } .s1 .slot:nth-child(4) .prev { height: 68px; } .s1 .slot:nth-child(5) .cur { height: 171px; } .s1 .slot:nth-child(5) .prev { height: 62px; } .s1 .slot:nth-child(6) .cur { height: 145px; } .s1 .slot:nth-child(6) .prev { height: 76px; }
.s1 .slot:nth-child(7) .cur { height: 119px; } .s1 .slot:nth-child(7) .prev { height: 94px; } .s1 .slot:nth-child(8) .cur { height: 97px; } .s1 .slot:nth-child(8) .prev { height: 108px; } .s1 .slot:nth-child(9) .cur { height: 85px; } .s1 .slot:nth-child(9) .prev { height: 119px; }
.s1 .slot:nth-child(10) .cur { height: 77px; } .s1 .slot:nth-child(10) .prev { height: 155px; } .s1 .slot:nth-child(11) .cur { height: 95px; } .s1 .slot:nth-child(11) .prev { height: 147px; } .s1 .slot:nth-child(12) .cur { height: 117px; } .s1 .slot:nth-child(12) .prev { height: 131px; }
.s1 .slot:nth-child(13) .cur { height: 135px; } .s1 .slot:nth-child(13) .prev { height: 110px; } .s1 .slot:nth-child(14) .cur { height: 149px; } .s1 .slot:nth-child(14) .prev { height: 90px; } .s1 .slot:nth-child(15) .cur { height: 194px; } .s1 .slot:nth-child(15) .prev { height: 72px; }
.s1 .slot:nth-child(16) .cur { height: 184px; } .s1 .slot:nth-child(16) .prev { height: 62px; } .s1 .slot:nth-child(17) .cur { height: 164px; } .s1 .slot:nth-child(17) .prev { height: 56px; } .s1 .slot:nth-child(18) .cur { height: 138px; } .s1 .slot:nth-child(18) .prev { height: 70px; }
.s1 .slot:nth-child(19) .cur { height: 112px; } .s1 .slot:nth-child(19) .prev { height: 88px; } .s1 .slot:nth-child(20) .cur { height: 90px; } .s1 .slot:nth-child(20) .prev { height: 114px; } .s1 .slot:nth-child(21) .cur { height: 78px; } .s1 .slot:nth-child(21) .prev { height: 125px; }
.s1 .slot:nth-child(22) .cur { height: 70px; } .s1 .slot:nth-child(22) .prev { height: 128px; } .s1 .slot:nth-child(23) .cur { height: 88px; } .s1 .slot:nth-child(23) .prev { height: 120px; } .s1 .slot:nth-child(24) .cur { height: 110px; } .s1 .slot:nth-child(24) .prev { height: 137px; }
.s2 .slot:nth-child(1) .cur { height: 194px; } .s2 .slot:nth-child(1) .prev { height: 70px; } .s2 .slot:nth-child(2) .cur { height: 174px; } .s2 .slot:nth-child(2) .prev { height: 64px; } .s2 .slot:nth-child(3) .cur { height: 148px; } .s2 .slot:nth-child(3) .prev { height: 111px; }
.s2 .slot:nth-child(4) .cur { height: 122px; } .s2 .slot:nth-child(4) .prev { height: 129px; } .s2 .slot:nth-child(5) .cur { height: 100px; } .s2 .slot:nth-child(5) .prev { height: 143px; } .s2 .slot:nth-child(6) .cur { height: 88px; } .s2 .slot:nth-child(6) .prev { height: 154px; }
.s2 .slot:nth-child(7) .cur { height: 80px; } .s2 .slot:nth-child(7) .prev { height: 158px; } .s2 .slot:nth-child(8) .cur { height: 139px; } .s2 .slot:nth-child(8) .prev { height: 150px; } .s2 .slot:nth-child(9) .cur { height: 161px; } .s2 .slot:nth-child(9) .prev { height: 134px; }
.s2 .slot:nth-child(10) .cur { height: 179px; } .s2 .slot:nth-child(10) .prev { height: 113px; } .s2 .slot:nth-child(11) .cur { height: 193px; } .s2 .slot:nth-child(11) .prev { height: 92px; } .s2 .slot:nth-child(12) .cur { height: 197px; } .s2 .slot:nth-child(12) .prev { height: 74px; }
.s2 .slot:nth-child(13) .cur { height: 187px; } .s2 .slot:nth-child(13) .prev { height: 98px; } .s2 .slot:nth-child(14) .cur { height: 167px; } .s2 .slot:nth-child(14) .prev { height: 91px; } .s2 .slot:nth-child(15) .cur { height: 141px; } .s2 .slot:nth-child(15) .prev { height: 106px; }
.s2 .slot:nth-child(16) .cur { height: 115px; } .s2 .slot:nth-child(16) .prev { height: 123px; } .s2 .slot:nth-child(17) .cur { height: 93px; } .s2 .slot:nth-child(17) .prev { height: 138px; } .s2 .slot:nth-child(18) .cur { height: 122px; } .s2 .slot:nth-child(18) .prev { height: 149px; }
.s2 .slot:nth-child(19) .cur { height: 114px; } .s2 .slot:nth-child(19) .prev { height: 152px; } .s2 .slot:nth-child(20) .cur { height: 132px; } .s2 .slot:nth-child(20) .prev { height: 155px; } .s2 .slot:nth-child(21) .cur { height: 154px; } .s2 .slot:nth-child(21) .prev { height: 139px; }
.s2 .slot:nth-child(22) .cur { height: 172px; } .s2 .slot:nth-child(22) .prev { height: 118px; } .s2 .slot:nth-child(23) .cur { height: 186px; } .s2 .slot:nth-child(23) .prev { height: 98px; } .s2 .slot:nth-child(24) .cur { height: 190px; } .s2 .slot:nth-child(24) .prev { height: 80px; }
</style>

<div class="plot" class:s0={series.get() == 0} class:s1={series.get() == 1} class:s2={series.get() == 2} class:cmp={compare.get()}>
  {for slot in slots.clone() key=slot}<div class="slot" data-slot={slot.get().to_string()}>{if compare.get()}<div class="prev"></div>{/if}<div class="cur"></div></div>{/for}
</div>
