use crate::layout::{Control, Shape};
use std::collections::HashMap;
use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, LinearGradient,
    Paint, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode, Stroke, Transform,
};

const PRESS_SCALE: f32 = 0.94;

pub struct Renderer { pub pixmap: Pixmap }

impl Renderer {
    pub fn new(w: u32, h: u32) -> Option<Self> {
        Pixmap::new(w, h).map(|pixmap| Self { pixmap })
    }
    pub fn resize(&mut self, w: u32, h: u32) {
        if self.pixmap.width()!=w || self.pixmap.height()!=h {
            if let Some(p)=Pixmap::new(w,h){ self.pixmap=p; }
        }
    }
    pub fn clear(&mut self) {
        self.pixmap.fill(Color::TRANSPARENT);
    }

    pub fn draw_controls(&mut self, controls: &[Control],
                         anim: &HashMap<u32,f32>, scale: f32,
                         stick_offsets: &HashMap<u32,(f32,f32)>) {
        for c in controls {
            let press = anim.get(&c.id).copied().unwrap_or(0.0);
            let offset = stick_offsets.get(&c.id).copied().unwrap_or((0.0,0.0));
            self.draw_control(c, press, scale, offset);
        }
    }

    fn draw_control(&mut self, c: &Control, press: f32, scale: f32, offset: (f32,f32)) {
        let cx = c.x * scale;
        let cy = c.y * scale;
        let s  = scale;
        let ps = 1.0 - (1.0 - PRESS_SCALE) * press;
        let t  = Transform::from_scale(ps, ps)
            .post_translate(cx*(1.0-ps), cy*(1.0-ps));
        let [cr,cg,cb,ca] = c.color;
        let alpha = ca as f32 / 255.0;

        match &c.shape {
            Shape::Circle { radius } =>
                self.draw_circle(cx, cy, radius*s, press, t, &c.label, cr, cg, cb, alpha, offset, scale),
            Shape::RoundedRect { width, height, radius } =>
                self.draw_rrect_btn(cx, cy, width*s, height*s, radius*s, press, t, &c.label),
            Shape::DpadCross { arm_width, arm_length } =>
                self.draw_dpad(cx, cy, arm_width*s, arm_length*s, press, t),
            Shape::Ring { outer_radius, inner_radius } =>
                self.draw_ring(cx, cy, outer_radius*s, inner_radius*s, press, t, offset, scale),
        }
    }

    fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, press: f32, t: Transform,
                   label: &str, cr: u8, cg: u8, cb: u8, alpha: f32,
                   offset: (f32,f32), scale: f32) {
        let is_xbox  = label == "xbox";
        let is_abxy  = matches!(label,"Y"|"X"|"B"|"A");
        let is_stick = matches!(label,"L3"|"R3");

        // Stick knob moves with offset
        let (kcx, kcy) = if is_stick {
            (cx + offset.0 * scale, cy + offset.1 * scale)
        } else {
            (cx, cy)
        };

        // Shadow
        let sa = ((1.0-press*0.8)*45.0) as u8;
        if sa > 0 {
            let mut sp=Paint::default(); sp.anti_alias=true;
            sp.set_color_rgba8(0,0,0,sa);
            let mut pb=PathBuilder::new(); pb.push_circle(kcx,kcy+r*0.12,r*1.1);
            if let Some(p)=pb.finish(){
                self.pixmap.fill_path(&p,&sp,FillRule::Winding,Transform::identity(),None);
            }
        }

        // Body
        let body_a = ((alpha + press*0.18)*255.0).min(255.0) as u8;
        let mut bp=Paint::default(); bp.anti_alias=true; bp.blend_mode=BlendMode::SourceOver;

        if is_xbox {
            if let Some(g)=RadialGradient::new(
                Point::from_xy(kcx-r*0.2,kcy-r*0.25), Point::from_xy(kcx,kcy), r*1.05,
                vec![GradientStop::new(0.0,Color::from_rgba8(22,163,74,body_a)),
                     GradientStop::new(1.0,Color::from_rgba8(15,100,50,body_a))],
                SpreadMode::Pad,Transform::identity()){ bp.shader=g; }
        } else if is_abxy {
            let bg_a = ((0.12 + press*0.10)*255.0).min(255.0) as u8;
            if let Some(g)=RadialGradient::new(
                Point::from_xy(kcx-r*0.2,kcy-r*0.25), Point::from_xy(kcx,kcy), r*1.05,
                vec![GradientStop::new(0.0,Color::from_rgba8(31,41,55,bg_a)),
                     GradientStop::new(1.0,Color::from_rgba8(17,24,39,bg_a))],
                SpreadMode::Pad,Transform::identity()){ bp.shader=g; }
        } else {
            if let Some(g)=RadialGradient::new(
                Point::from_xy(kcx-r*0.22,kcy-r*0.28), Point::from_xy(kcx,kcy), r*1.05,
                vec![GradientStop::new(0.0,Color::from_rgba8(
                    (cr as f32*1.2).min(255.0) as u8,
                    (cg as f32*1.2).min(255.0) as u8,
                    (cb as f32*1.2).min(255.0) as u8, body_a)),
                     GradientStop::new(1.0,Color::from_rgba8(
                    (cr as f32*0.7) as u8,
                    (cg as f32*0.7) as u8,
                    (cb as f32*0.7) as u8, body_a))],
                SpreadMode::Pad,Transform::identity()){ bp.shader=g; }
        }
        let mut pb=PathBuilder::new(); pb.push_circle(kcx,kcy,r);
        if let Some(p)=pb.finish(){
            self.pixmap.fill_path(&p,&bp,FillRule::Winding,t,None);
        }

        // Border
        let sw = if is_abxy{2.5}else{1.5};
        let sa2= if is_xbox{220u8}else if is_abxy{200u8}else{100u8};
        let (bcr,bcg,bcb)=if is_xbox{(34u8,197u8,94u8)}else{(cr,cg,cb)};
        let mut rim=Paint::default(); rim.anti_alias=true;
        rim.set_color_rgba8(bcr,bcg,bcb,sa2);
        let sk=Stroke{width:sw,..Default::default()};
        let mut pb2=PathBuilder::new(); pb2.push_circle(kcx,kcy,r-sw/2.0);
        if let Some(p)=pb2.finish(){ self.pixmap.stroke_path(&p,&rim,&sk,t,None); }

        // Sheen
        if let Some(g)=LinearGradient::new(
            Point::from_xy(kcx,kcy-r), Point::from_xy(kcx,kcy-r*0.15),
            vec![GradientStop::new(0.0,Color::from_rgba8(255,255,255,30)),
                 GradientStop::new(1.0,Color::from_rgba8(255,255,255,0))],
            SpreadMode::Pad,Transform::identity()) {
            let mut hp=Paint::default();
            hp.anti_alias=true; hp.blend_mode=BlendMode::SourceOver; hp.shader=g;
            let mut pb3=PathBuilder::new(); pb3.push_circle(kcx,kcy,r-sw);
            if let Some(p)=pb3.finish(){
                self.pixmap.fill_path(&p,&hp,FillRule::Winding,t,None);
            }
        }

        // Press glow
        if press>0.05 && (is_abxy||is_xbox) {
            let ga=(press*0.35*255.0) as u8;
            if let Some(g)=RadialGradient::new(
                Point::from_xy(kcx,kcy), Point::from_xy(kcx,kcy), r*1.8,
                vec![GradientStop::new(0.0,Color::from_rgba8(bcr,bcg,bcb,ga)),
                     GradientStop::new(1.0,Color::from_rgba8(bcr,bcg,bcb,0))],
                SpreadMode::Pad,Transform::identity()) {
                let mut gp=Paint::default();
                gp.anti_alias=true; gp.blend_mode=BlendMode::SourceOver; gp.shader=g;
                let mut pb4=PathBuilder::new(); pb4.push_circle(kcx,kcy,r*1.8);
                if let Some(p)=pb4.finish(){
                    self.pixmap.fill_path(&p,&gp,FillRule::Winding,Transform::identity(),None);
                }
            }
        }

        // Xbox diamond
        if is_xbox {
            let ds=r*0.38;
            let mut dp=Paint::default(); dp.anti_alias=true;
            dp.set_color_rgba8(236,253,245,215);
            let mut pb5=PathBuilder::new();
            pb5.move_to(kcx,kcy-ds); pb5.line_to(kcx+ds,kcy);
            pb5.line_to(kcx,kcy+ds); pb5.line_to(kcx-ds,kcy); pb5.close();
            if let Some(p)=pb5.finish(){
                self.pixmap.fill_path(&p,&dp,FillRule::Winding,t,None);
            }
        }

        // Select icon
        if label=="sel" {
            let mut ip=Paint::default(); ip.anti_alias=true;
            ip.set_color_rgba8(180,190,200,200);
            let sq=Stroke{width:1.6,..Default::default()};
            for (ox,oy) in [(-3.5f32,-3.5f32),(2.5,2.5)] {
                if let Some(path)=Self::rrect(cx+ox,cy+oy,9.0,9.0,1.5){
                    self.pixmap.stroke_path(&path,&ip,&sq,t,None);
                }
            }
        }

        // Menu icon
        if label=="men" {
            let mut lp=Paint::default(); lp.anti_alias=true;
            lp.set_color_rgba8(180,190,200,200);
            let ls=Stroke{width:1.6,line_cap:tiny_skia::LineCap::Round,..Default::default()};
            for dy in [-4.5f32,0.0,4.5] {
                let mut pb=PathBuilder::new();
                pb.move_to(cx-7.5,cy+dy); pb.line_to(cx+7.5,cy+dy);
                if let Some(p)=pb.finish(){ self.pixmap.stroke_path(&p,&lp,&ls,t,None); }
            }
        }

        // ABXY colored letter
        if is_abxy {
            self.draw_letter_colored(label,kcx,kcy,r*0.58,t,cr,cg,cb);
        }

        // L3/R3 below
        if is_stick {
            self.draw_small_label(label,cx,cy+r+11.0,r*0.28,t);
        }
    }

    fn draw_rrect_btn(&mut self, cx:f32, cy:f32, w:f32, h:f32, r:f32,
                      press:f32, t:Transform, label:&str) {
        let x0=cx-w/2.0; let y0=cy-h/2.0;
        let sa=((1.0-press*0.8)*40.0) as u8;
        if sa>0 {
            let mut sp=Paint::default(); sp.anti_alias=true;
            sp.set_color_rgba8(0,0,0,sa);
            if let Some(p)=Self::rrect(x0+1.5,y0+3.0,w,h,r){
                self.pixmap.fill_path(&p,&sp,FillRule::Winding,Transform::identity(),None);
            }
        }

        let bright=(press*30.0) as u8;
        let base=[25u8,35,60];
        let hi=[(base[0]+bright).min(255),(base[1]+bright).min(255),(base[2]+(bright as f32*1.4) as u8).min(255)];
        let lo=[base[0].saturating_add(bright/2),base[1].saturating_add(bright/2),base[2].saturating_add(bright)];

        if let Some(g)=LinearGradient::new(
            Point::from_xy(cx,y0), Point::from_xy(cx,y0+h),
            vec![GradientStop::new(0.0,Color::from_rgba8(hi[0],hi[1],hi[2],235)),
                 GradientStop::new(1.0,Color::from_rgba8(lo[0],lo[1],lo[2],235))],
            SpreadMode::Pad,Transform::identity()) {
            let mut bp=Paint::default();
            bp.anti_alias=true; bp.blend_mode=BlendMode::SourceOver; bp.shader=g;
            if let Some(p)=Self::rrect(x0,y0,w,h,r){
                self.pixmap.fill_path(&p,&bp,FillRule::Winding,t,None);
            }
        }

        let mut rim=Paint::default(); rim.anti_alias=true;
        rim.set_color_rgba8(55,70,95,200);
        let sk=Stroke{width:1.0,..Default::default()};
        if let Some(p)=Self::rrect(x0+0.5,y0+0.5,w-1.0,h-1.0,r){
            self.pixmap.stroke_path(&p,&rim,&sk,t,None);
        }

        if let Some(g)=LinearGradient::new(
            Point::from_xy(cx,y0), Point::from_xy(cx,y0+h*0.5),
            vec![GradientStop::new(0.0,Color::from_rgba8(255,255,255,20)),
                 GradientStop::new(1.0,Color::from_rgba8(255,255,255,0))],
            SpreadMode::Pad,Transform::identity()) {
            let mut hp=Paint::default();
            hp.anti_alias=true; hp.blend_mode=BlendMode::SourceOver; hp.shader=g;
            if let Some(p)=Self::rrect(x0+1.0,y0+1.0,w-2.0,h*0.5,r){
                self.pixmap.fill_path(&p,&hp,FillRule::Winding,t,None);
            }
        }

        self.draw_small_label(label,cx,cy,h*0.44,t);
    }

    fn draw_dpad(&mut self, cx:f32, cy:f32, arm_w:f32, arm_l:f32, press:f32, t:Transform) {
        let bright=(press*25.0) as u8;
        let base=[25u8,35,60];
        let hi=[(base[0]+bright).min(255),(base[1]+bright).min(255),(base[2]+(bright as f32*1.5) as u8).min(255)];
        let lo=[base[0],base[1],base[2]];

        for (x,y,w,h) in [
            (cx-arm_l,cy-arm_w,arm_l*2.0,arm_w*2.0),
            (cx-arm_w,cy-arm_l,arm_w*2.0,arm_l*2.0),
        ] {
            let sa=((1.0-press*0.8)*35.0) as u8;
            if sa>0 {
                let mut sp=Paint::default(); sp.anti_alias=true;
                sp.set_color_rgba8(0,0,0,sa);
                if let Some(rect)=tiny_skia::Rect::from_xywh(x+1.0,y+2.0,w,h){
                    let mut pb=PathBuilder::new(); pb.push_rect(rect);
                    if let Some(p)=pb.finish(){
                        self.pixmap.fill_path(&p,&sp,FillRule::Winding,Transform::identity(),None);
                    }
                }
            }
            if let Some(g)=LinearGradient::new(
                Point::from_xy(cx,y), Point::from_xy(cx,y+h),
                vec![GradientStop::new(0.0,Color::from_rgba8(hi[0],hi[1],hi[2],120)),
                     GradientStop::new(1.0,Color::from_rgba8(lo[0],lo[1],lo[2],120))],
                SpreadMode::Pad,Transform::identity()) {
                let mut bp=Paint::default();
                bp.anti_alias=true; bp.blend_mode=BlendMode::SourceOver; bp.shader=g;
                if let Some(path)=Self::rrect(x,y,w,h,5.0){
                    self.pixmap.fill_path(&path,&bp,FillRule::Winding,t,None);
                }
            }
            let mut rim=Paint::default(); rim.anti_alias=true;
            rim.set_color_rgba8(55,70,95,190);
            let sk=Stroke{width:1.0,..Default::default()};
            if let Some(path)=Self::rrect(x+0.5,y+0.5,w-1.0,h-1.0,5.0){
                self.pixmap.stroke_path(&path,&rim,&sk,t,None);
            }
        }
        self.draw_dpad_arrows(cx,cy,arm_l,arm_w,t);
    }

    fn draw_dpad_arrows(&mut self, cx:f32, cy:f32, arm_l:f32, arm_w:f32, t:Transform) {
        let tip_off=arm_l*0.74; let base_off=arm_l*0.50; let base_w=arm_w*0.68;
        let mut ap=Paint::default(); ap.anti_alias=true;
        ap.set_color_rgba8(140,155,175,200);
        for (tx,ty,blx,bly,brx,bry) in [
            ( 0.0,    -tip_off,-base_w,-base_off, base_w,-base_off),
            ( 0.0,     tip_off,-base_w, base_off, base_w, base_off),
            (-tip_off, 0.0,   -base_off,-base_w, -base_off, base_w),
            ( tip_off, 0.0,    base_off,-base_w,  base_off, base_w),
        ] {
            let mut pb=PathBuilder::new();
            pb.move_to(cx+tx,cy+ty); pb.line_to(cx+blx,cy+bly);
            pb.line_to(cx+brx,cy+bry); pb.close();
            if let Some(p)=pb.finish(){
                self.pixmap.fill_path(&p,&ap,FillRule::Winding,t,None);
            }
        }
    }

    fn draw_ring(&mut self, cx:f32, cy:f32, outer:f32, inner:f32,
                 press:f32, t:Transform, offset:(f32,f32), scale:f32) {
        // Outer ring (static, doesn't move)
        let mut op=Paint::default(); op.anti_alias=true;
        op.set_color_rgba8(20,28,42,100);
        let mut pb=PathBuilder::new(); pb.push_circle(cx,cy,outer);
        if let Some(p)=pb.finish(){ self.pixmap.fill_path(&p,&op,FillRule::Winding,t,None); }

        let mut rs=Paint::default(); rs.anti_alias=true;
        rs.set_color_rgba8(55,70,95,200);
        let rsk=Stroke{width:1.5,..Default::default()};
        let mut pb2=PathBuilder::new(); pb2.push_circle(cx,cy,outer-0.75);
        if let Some(p)=pb2.finish(){ self.pixmap.stroke_path(&p,&rs,&rsk,t,None); }

        // Inner shadow vignette
        if let Some(g)=RadialGradient::new(
            Point::from_xy(cx,cy), Point::from_xy(cx,cy), outer,
            vec![GradientStop::new(0.0,Color::from_rgba8(0,0,0,0)),
                 GradientStop::new(0.65,Color::from_rgba8(0,0,0,0)),
                 GradientStop::new(1.0,Color::from_rgba8(0,0,0,60))],
            SpreadMode::Pad,Transform::identity()) {
            let mut ip=Paint::default();
            ip.anti_alias=true; ip.blend_mode=BlendMode::SourceOver; ip.shader=g;
            let mut pb3=PathBuilder::new(); pb3.push_circle(cx,cy,outer-1.0);
            if let Some(p)=pb3.finish(){
                self.pixmap.fill_path(&p,&ip,FillRule::Winding,t,None);
            }
        }

        // Knob moves with offset
        let kcx = cx + offset.0 * scale;
        let kcy = cy + offset.1 * scale;

        if let Some(g)=RadialGradient::new(
            Point::from_xy(kcx-inner*0.2,kcy-inner*0.25), Point::from_xy(kcx,kcy), inner,
            vec![GradientStop::new(0.0,Color::from_rgba8(175,182,190,255)),
                 GradientStop::new(0.55,Color::from_rgba8(148,156,165,255)),
                 GradientStop::new(1.0,Color::from_rgba8(105,112,120,255))],
            SpreadMode::Pad,Transform::identity()) {
            let mut kp=Paint::default();
            kp.anti_alias=true; kp.blend_mode=BlendMode::SourceOver; kp.shader=g;
            let mut pb4=PathBuilder::new(); pb4.push_circle(kcx,kcy,inner);
            if let Some(p)=pb4.finish(){
                self.pixmap.fill_path(&p,&kp,FillRule::Winding,t,None);
            }
        }

        // Knob border
        let mut kb=Paint::default(); kb.anti_alias=true;
        kb.set_color_rgba8(80,88,96,180);
        let kbs=Stroke{width:1.2,..Default::default()};
        let mut pb5=PathBuilder::new(); pb5.push_circle(kcx,kcy,inner-0.6);
        if let Some(p)=pb5.finish(){ self.pixmap.stroke_path(&p,&kb,&kbs,t,None); }

        // Knob sheen
        if let Some(g)=LinearGradient::new(
            Point::from_xy(kcx,kcy-inner), Point::from_xy(kcx,kcy-inner*0.1),
            vec![GradientStop::new(0.0,Color::from_rgba8(255,255,255,40)),
                 GradientStop::new(1.0,Color::from_rgba8(255,255,255,0))],
            SpreadMode::Pad,Transform::identity()) {
            let mut hp=Paint::default();
            hp.anti_alias=true; hp.blend_mode=BlendMode::SourceOver; hp.shader=g;
            let mut pb6=PathBuilder::new(); pb6.push_circle(kcx,kcy,inner*0.88);
            if let Some(p)=pb6.finish(){
                self.pixmap.fill_path(&p,&hp,FillRule::Winding,t,None);
            }
        }

        // Center dimple
        let mut dp=Paint::default(); dp.anti_alias=true;
        dp.set_color_rgba8(50,57,66,200);
        let mut pb7=PathBuilder::new(); pb7.push_circle(kcx,kcy,inner*0.20);
        if let Some(p)=pb7.finish(){
            self.pixmap.fill_path(&p,&dp,FillRule::Winding,t,None);
        }

        let _ = press;
    }

    fn draw_letter_colored(&mut self, label:&str, cx:f32, cy:f32, size:f32,
                           t:Transform, cr:u8, cg:u8, cb:u8) {
        let mut tp=Paint::default(); tp.anti_alias=true;
        tp.set_color_rgba8(cr,cg,cb,235);
        tp.blend_mode=BlendMode::SourceOver;
        let sw=(size*0.20).max(2.0);
        let stroke=Stroke{
            width:sw,
            line_cap:tiny_skia::LineCap::Round,
            line_join:tiny_skia::LineJoin::Round,
            ..Default::default()
        };
        for pb in Self::letter_stroke(label,cx,cy,size) {
            if let Some(p)=pb.finish(){
                self.pixmap.stroke_path(&p,&tp,&stroke,t,None);
            }
        }
    }

    fn letter_stroke(label:&str, cx:f32, cy:f32, s:f32) -> Vec<PathBuilder> {
        let h=s*0.60; let w=s*0.40;
        let mut v=Vec::new();
        match label {
            "A" => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w,cy+h); p.line_to(cx,cy-h); p.line_to(cx+w,cy+h);
                p.move_to(cx-w*0.55,cy+h*0.10); p.line_to(cx+w*0.55,cy+h*0.10);
                v.push(p);
            }
            "B" => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w*0.5,cy-h); p.line_to(cx-w*0.5,cy+h);
                p.move_to(cx-w*0.5,cy-h);
                p.quad_to(cx+w*1.1,cy-h, cx+w*1.1,cy-h*0.15);
                p.quad_to(cx+w*1.1,cy,   cx-w*0.5,cy);
                p.move_to(cx-w*0.5,cy);
                p.quad_to(cx+w*1.2,cy,   cx+w*1.2,cy+h*0.5);
                p.quad_to(cx+w*1.2,cy+h, cx-w*0.5,cy+h);
                v.push(p);
            }
            "X" => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w,cy-h); p.line_to(cx+w,cy+h);
                p.move_to(cx+w,cy-h); p.line_to(cx-w,cy+h);
                v.push(p);
            }
            "Y" => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w,cy-h); p.line_to(cx,cy-h*0.05);
                p.line_to(cx+w,cy-h);
                p.move_to(cx,cy-h*0.05); p.line_to(cx,cy+h);
                v.push(p);
            }
            _ => {}
        }
        v
    }

    fn draw_small_label(&mut self, label:&str, cx:f32, cy:f32, size:f32, t:Transform) {
        if matches!(label,""|"xbox"|"sel"|"men") { return; }
        let mut tp=Paint::default(); tp.anti_alias=true;
        tp.set_color_rgba8(210,218,228,220);
        tp.blend_mode=BlendMode::SourceOver;
        let sw=(size*0.20).max(1.8);
        let stroke=Stroke{
            width:sw,
            line_cap:tiny_skia::LineCap::Round,
            line_join:tiny_skia::LineJoin::Round,
            ..Default::default()
        };
        let chars:Vec<char>=label.chars().collect();
        let char_w=size*0.72;
        let total_w=char_w*(chars.len() as f32-1.0);
        for (i,&ch) in chars.iter().enumerate() {
            let lx=cx-total_w/2.0+i as f32*char_w;
            for pb in Self::glyph(ch,lx,cy,size) {
                if let Some(p)=pb.finish(){
                    self.pixmap.stroke_path(&p,&tp,&stroke,t,None);
                }
            }
        }
    }

    fn glyph(ch:char, cx:f32, cy:f32, s:f32) -> Vec<PathBuilder> {
        let h=s*0.56; let w=s*0.36;
        let mut v=Vec::new();
        match ch {
            'L' => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w*0.3,cy-h); p.line_to(cx-w*0.3,cy+h);
                p.line_to(cx+w*0.7,cy+h); v.push(p);
            }
            'T' => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w,cy-h); p.line_to(cx+w,cy-h);
                p.move_to(cx,cy-h); p.line_to(cx,cy+h); v.push(p);
            }
            'R' => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w*0.4,cy-h); p.line_to(cx-w*0.4,cy+h);
                p.move_to(cx-w*0.4,cy-h);
                p.quad_to(cx+w,cy-h, cx+w,cy-h*0.1);
                p.quad_to(cx+w,cy,   cx-w*0.4,cy);
                p.line_to(cx+w*0.75,cy+h); v.push(p);
            }
            'B' => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w*0.5,cy-h); p.line_to(cx-w*0.5,cy+h);
                p.move_to(cx-w*0.5,cy-h);
                p.quad_to(cx+w*1.1,cy-h, cx+w*1.1,cy);
                p.quad_to(cx+w*1.1,cy+h, cx-w*0.5,cy+h);
                p.move_to(cx-w*0.5,cy); p.line_to(cx+w*0.9,cy);
                v.push(p);
            }
            '3' => {
                let mut p=PathBuilder::new();
                p.move_to(cx-w*0.6,cy-h);
                p.quad_to(cx+w,cy-h, cx+w,cy-h*0.3);
                p.quad_to(cx+w,cy,   cx-w*0.1,cy);
                p.move_to(cx-w*0.1,cy);
                p.quad_to(cx+w,cy, cx+w,cy+h*0.3);
                p.quad_to(cx+w,cy+h, cx-w*0.6,cy+h);
                v.push(p);
            }
            _ => {}
        }
        v
    }

    fn rrect(x0:f32,y0:f32,w:f32,h:f32,r:f32)->Option<tiny_skia::Path>{
        let r=r.min(w/2.0).min(h/2.0);
        if w<=0.0||h<=0.0{return None;}
        let mut pb=PathBuilder::new();
        pb.move_to(x0+r,y0); pb.line_to(x0+w-r,y0);
        pb.quad_to(x0+w,y0,   x0+w,y0+r);
        pb.line_to(x0+w,y0+h-r);
        pb.quad_to(x0+w,y0+h, x0+w-r,y0+h);
        pb.line_to(x0+r,y0+h);
        pb.quad_to(x0,y0+h,   x0,y0+h-r);
        pb.line_to(x0,y0+r);
        pb.quad_to(x0,y0,     x0+r,y0);
        pb.close(); pb.finish()
    }

    pub fn to_argb_buffer(&self, buffer: &mut [u32]) {
        let data = self.pixmap.data();
        let len = buffer.len().min(data.len() / 4);
        let src_chunks = data[..len * 4].chunks_exact(4);
        let dst_pixels = &mut buffer[..len];

        for (pixel, chunk) in dst_pixels.iter_mut().zip(src_chunks) {
            let r = chunk[0] as u32;
            let g = chunk[1] as u32;
            let b = chunk[2] as u32;
            let a = chunk[3] as u32;
            *pixel = (a << 24) | (r << 16) | (g << 8) | b;
        }
    }
}
