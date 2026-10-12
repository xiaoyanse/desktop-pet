use super::*;

#[test]
fn selection_clips_reverse_drag_to_negative_desktop_and_rejects_empty() {
    let bounds = Region {
        left: -100,
        top: -20,
        width: 200,
        height: 100,
    };
    assert_eq!(
        Selection::rectangle([90, 50], [-120, -30], bounds)
            .unwrap()
            .region,
        Region {
            left: -100,
            top: -20,
            width: 190,
            height: 70
        }
    );
    assert!(Selection::rectangle([0, 0], [0, 8], bounds).is_none());
    assert!(
        Region {
            left: i32::MAX,
            top: 0,
            width: 1,
            height: 1
        }
        .bytes()
        .is_err()
    );
    assert!(
        Region {
            left: 0,
            top: 0,
            width: 16384,
            height: 16384
        }
        .bytes()
        .is_err()
    );
}
#[test]
fn freehand_masks_outside_triangle_and_preserves_inside_original() {
    let region = Region {
        left: -4,
        top: 10,
        width: 4,
        height: 4,
    };
    let selection = Selection::freehand(
        vec![[-4, 10], [0, 10], [-4, 14]],
        Region {
            left: -4,
            top: 10,
            width: 5,
            height: 5,
        },
    )
    .unwrap();
    assert_eq!(selection.region, region);
    let original = Image::new([4, 4], vec![255; 64]).unwrap();
    let result = selection.apply_mask(original.clone()).unwrap();
    assert_eq!(&result.pixels()[0..4], &[255; 4]);
    assert_eq!(&result.pixels()[60..64], &[0; 4]);
    assert!(original.pixels().iter().all(|v| *v == 255));
    assert!(Selection::freehand(vec![[0, 0]; 2], region).is_err());
}

#[test]
fn freehand_rejects_zero_area_and_render_budget_keeps_last_valid_annotation() {
    let bounds = Region {
        left: 0,
        top: 0,
        width: 16,
        height: 16,
    };
    let path = Selection::freehand(vec![[0, 0], [4, 4], [8, 8]], bounds).unwrap();
    assert!(
        path.apply_mask(Image::new([8, 8], vec![255; 256]).unwrap())
            .is_err()
    );
    let mut doc = Document::new(Image::new([512, 512], vec![255; 512 * 512 * 4]).unwrap());
    let long = Mark {
        tool: Tool::Pen,
        points: (0..1024)
            .map(|i| if i % 2 == 0 { [0, 0] } else { [511, 511] })
            .collect(),
        color: [0, 0, 0],
        width: 24,
    };
    assert!(doc.add(long).is_err());
    assert!(doc.marks().is_empty());
}
#[test]
fn annotations_undo_redo_and_export_never_mutate_original() {
    let mut doc = Document::new(Image::new([8, 8], vec![255; 256]).unwrap());
    let mark = Mark {
        tool: Tool::Cover,
        points: vec![[2, 2], [4, 4]],
        color: [0, 0, 0],
        width: 1,
    };
    doc.add(mark.clone()).unwrap();
    assert_eq!(&doc.render().unwrap().pixels()[72..76], &[0, 0, 0, 255]);
    assert_eq!(&doc.image.pixels()[72..76], &[255; 4]);
    doc.undo();
    assert!(doc.can_redo());
    assert!(doc.marks().is_empty());
    doc.redo();
    assert!(doc.can_undo());
    doc.undo();
    doc.add(mark).unwrap();
    assert!(!doc.can_redo());
    assert!(
        doc.add(Mark {
            tool: Tool::Pen,
            points: vec![[9, 0], [1, 1]],
            color: [0, 0, 0],
            width: 2
        })
        .is_err()
    );
}
