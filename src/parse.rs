use crate::array::{variable, Builder, PrimitiveBuilder, StringBuilder};
use crate::csv::{reader::*, FieldParser, Record};
use crate::datatypes::*;
use crate::Column;
use dashmap::DashMap;
use num_traits::ToPrimitive;
use std::sync::Arc;

type ConcurrentEnumMaps = Arc<DashMap<usize, Arc<DashMap<String, (u32, usize)>>>>;

pub fn records_to_columns(
    values: &[&[u8]],
    parsers: &[FieldParser],
    labels: &ConcurrentEnumMaps,
) -> Result<Vec<Column>, variable::Error> {
    let values = Record::from_data(values);
    let mut batch = Vec::with_capacity(parsers.len());
    for (i, parser) in parsers.iter().enumerate() {
        let col = match parser {
            FieldParser::Int64(parse) | FieldParser::Timestamp(parse) => {
                build_primitive_array::<Int64Type, Int64Parser>(&values, i, parse)?
            }
            FieldParser::Float64(parse) => {
                build_primitive_array::<Float64Type, Float64Parser>(&values, i, parse)?
            }
            FieldParser::Utf8 => {
                let mut builder = StringBuilder::with_capacity(values.len())?;
                for row in &values {
                    builder.try_push(std::str::from_utf8(row.get(i).unwrap_or_default())?)?;
                }
                builder.build()
            }
            FieldParser::UInt32(parse) => {
                build_primitive_array::<UInt32Type, UInt32Parser>(&values, i, parse)?
            }
            FieldParser::Dict => {
                let mut builder = PrimitiveBuilder::<UInt32Type>::with_capacity(values.len())?;
                for r in &values {
                    let key = std::str::from_utf8(r.get(i).unwrap_or_default())?;
                    let value = labels.get(&i).map_or_else(u32::max_value, |map| {
                        let enum_value = map
                            .get_or_insert(
                                &key.to_string(),
                                (
                                    (map.len() + 1).to_u32().unwrap_or(u32::max_value()),
                                    0_usize,
                                ),
                            )
                            .0;
                        map.alter(key, |v| (v.0, v.1 + 1));
                        enum_value
                        // u32::max_value means something wrong, and 0 means unmapped. And, enum value starts with 1.
                    });
                    builder.try_push(value)?;
                }
                builder.build()
            }
        };
        batch.push(col.into());
    }
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{Array, StringArray};
    use chrono::{NaiveDate, NaiveDateTime};
    use itertools::izip;
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::net::Ipv4Addr;

    pub fn convert_to_conc_enum_maps(
        enum_maps: &HashMap<usize, HashMap<String, (u32, usize)>>,
    ) -> ConcurrentEnumMaps {
        let c_enum_maps = Arc::new(DashMap::default());

        for (column, map) in enum_maps {
            let c_map = Arc::new(DashMap::<String, (u32, usize)>::default());
            for (data, enum_val) in map {
                c_map.insert(data.clone(), (enum_val.0, enum_val.1));
            }
            c_enum_maps.insert(*column, c_map);
        }
        c_enum_maps
    }

    fn get_test_data() -> (
        Vec<Vec<u8>>,
        HashMap<usize, HashMap<String, (u32, usize)>>,
        Vec<Column>,
    ) {
        let c0_v: Vec<i64> = vec![1, 3, 3, 5, 2, 1, 3];
        let c1_v: Vec<_> = vec!["111a qwer", "b", "c", "d", "b", "111a qwer", "111a qwer"];
        let c2_v: Vec<Ipv4Addr> = vec![
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(127, 0, 0, 2),
            Ipv4Addr::new(127, 0, 0, 3),
            Ipv4Addr::new(127, 0, 0, 4),
            Ipv4Addr::new(127, 0, 0, 2),
            Ipv4Addr::new(127, 0, 0, 2),
            Ipv4Addr::new(127, 0, 0, 3),
        ];
        let c3_v: Vec<f64> = vec![2.2, 3.14, 122.8, 5.3123, 7.0, 10320.811, 5.5];
        let c4_v: Vec<NaiveDateTime> = vec![
            NaiveDate::from_ymd(2019, 9, 22).and_hms(6, 10, 11),
            NaiveDate::from_ymd(2019, 9, 22).and_hms(6, 15, 11),
            NaiveDate::from_ymd(2019, 9, 21).and_hms(20, 10, 11),
            NaiveDate::from_ymd(2019, 9, 21).and_hms(20, 10, 11),
            NaiveDate::from_ymd(2019, 9, 22).and_hms(6, 45, 11),
            NaiveDate::from_ymd(2019, 9, 21).and_hms(8, 10, 11),
            NaiveDate::from_ymd(2019, 9, 22).and_hms(9, 10, 11),
        ];
        let c5_v: Vec<u32> = vec![1, 2, 2, 2, 2, 2, 7];

        let mut c5_map: HashMap<u32, String> = HashMap::new();
        c5_map.insert(1, "t1".to_string());
        c5_map.insert(2, "t2".to_string());
        c5_map.insert(7, "t3".to_string());

        let mut data = vec![];
        let fmt = "%Y-%m-%d %H:%M:%S";
        for (c0, c1, c2, c3, c4, c5) in izip!(
            c0_v.iter(),
            c1_v.iter(),
            c2_v.iter(),
            c3_v.iter(),
            c4_v.iter(),
            c5_v.iter()
        ) {
            let mut row: Vec<u8> = vec![];
            row.extend(c0.to_string().into_bytes());
            row.extend_from_slice(b",");
            row.extend(c1.to_string().into_bytes());
            row.extend_from_slice(b",");
            row.extend(c2.to_string().into_bytes());
            row.extend_from_slice(b",");
            row.extend(c3.to_string().into_bytes());
            row.extend_from_slice(b",");
            row.extend(c4.format(fmt).to_string().into_bytes());
            row.extend_from_slice(b",");
            row.extend(c5_map.get(c5).unwrap().to_string().into_bytes());
            data.push(row);
        }

        let mut labels = HashMap::new();
        labels.insert(5, c5_map.into_iter().map(|(k, v)| (v, (k, 0))).collect());

        let c0 = Column::try_from_slice::<Int64Type>(&c0_v).unwrap();
        let c1_a: Arc<dyn Array> = Arc::new(StringArray::try_from(c1_v.as_slice()).unwrap());
        let c1 = Column::from(c1_a);
        let c2 = Column::try_from_slice::<UInt32Type>(
            c2_v.iter()
                .map(|&v| -> u32 { v.into() })
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .unwrap();
        let c3 = Column::try_from_slice::<Float64Type>(&c3_v).unwrap();
        let c4 = Column::try_from_slice::<Int64Type>(
            c4_v.iter()
                .map(|v| v.timestamp())
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .unwrap();
        let c5 = Column::try_from_slice::<UInt32Type>(&c5_v).unwrap();
        let columns: Vec<Column> = vec![c0, c1, c2, c3, c4, c5];
        (data, labels, columns)
    }

    #[test]
    fn parse_records() {
        let parsers = [
            FieldParser::int64(),
            FieldParser::Utf8,
            FieldParser::uint32_with_parser(|v| {
                let val: String = v.iter().map(|&c| c as char).collect();
                val.parse::<Ipv4Addr>().map(Into::into).map_err(Into::into)
            }),
            FieldParser::float64(),
            FieldParser::timestamp_with_parser(move |v| {
                let val: String = v.iter().map(|&c| c as char).collect();
                Ok(NaiveDateTime::parse_from_str(&val, "%Y-%m-%d %H:%M:%S")?.timestamp())
            }),
            FieldParser::Dict,
        ];
        let (data, labels, columns) = get_test_data();
        let records: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
        let result =
            super::records_to_columns(&records, &parsers, &convert_to_conc_enum_maps(&labels))
                .unwrap();
        assert_eq!(result, columns);
    }

    #[test]
    fn missing_enum_map() {
        let parsers = [FieldParser::Dict];
        let labels = HashMap::<usize, HashMap<String, (u32, usize)>>::new();

        let record = "1\n".to_string().into_bytes();
        let row = vec![record.as_slice()];
        let result = super::records_to_columns(
            row.as_slice(),
            &parsers,
            &convert_to_conc_enum_maps(&labels),
        )
        .unwrap();

        let c = Column::try_from_slice::<UInt32Type>(&[u32::max_value()][0..1]).unwrap();
        assert_eq!(c, result[0]);
    }
}
