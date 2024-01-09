/*
https://ptb.discord.com/channels/635447594499178496/1144154882811646012/1229780619102257194
<Area>
  <Name>鹿児島県大隅</Name>
  <Code>771</Code>
  <MaxInt>2</MaxInt>
</Area>
<Area>
  <Name>鹿児島県奄美北部</Name>
  <Code>778</Code>
  <MaxInt>2</MaxInt>
</Area>
<Area>
  <Name>鹿児島県薩摩</Name>
  <Code>770</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>鹿児島県十島村</Name>
  <Code>774</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>鹿児島県種子島</Name>
  <Code>776</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>鹿児島県屋久島</Name>
  <Code>777</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>鹿児島県奄美南部</Name>
  <Code>779</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>宮崎県北部山沿い</Name>
  <Code>761</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>宮崎県南部平野部</Name>
  <Code>762</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>宮崎県南部山沿い</Name>
  <Code>763</Code>
  <MaxInt>1</MaxInt>
</Area>
<Area>
  <Name>沖縄県大東島</Name>
  <Code>803</Code>
  <MaxInt>1</MaxInt>
</Area>
 */

use crate::intensity::震度;

pub struct Area {
    pub code: u32,
    pub intensity: 震度,
}

impl Area {
    const fn new(code: u32, intensity: 震度) -> Self {
        Self { code, intensity }
    }
}

// pub const EARTHQUAKE_DATA: EnumMap<震度, Vec<u32>> = enum_map! {
//     震度1 => vec![770, 774, 776, 777, 779, 761, 762, 763, 803],
//     震度2 => vec![771, 778],
// };

// pub const EARTHQUAKE_DATA: [Area; 11] = [
//     Area::new(771, 震度2),
//     Area::new(778, 震度2),
//     Area::new(770, 震度1),
//     Area::new(774, 震度1),
//     Area::new(776, 震度1),
//     Area::new(777, 震度1),
//     Area::new(779, 震度1),
//     Area::new(761, 震度1),
//     Area::new(762, 震度1),
//     Area::new(763, 震度1),
//     Area::new(803, 震度1),
// ];
