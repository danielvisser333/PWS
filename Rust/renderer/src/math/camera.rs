use ash::vk::Extent2D;
use cgmath::{Matrix4, Deg, Point3, Vector3, Quaternion, InnerSpace};

use super::UniformBuffer;

pub struct Camera{
    view : Matrix4<f32>,
    projection : Matrix4<f32>,
    near : f32,
    far : f32,
    fov : f32,
    aspect : f32,
    eye : Point3<f32>,
    center : Point3<f32>,
    up : Vector3<f32>,
    pub matrix : UniformBuffer,
    pub left_mouse_button_pressed : bool,
}
impl Camera{
    pub fn new(extent : Extent2D) -> Self{
        let near = 0.1;
        let far = 10.0;
        let fov = 70.0;
        let aspect = extent.width as f32 / extent.height as f32;
        let mut projection = cgmath::perspective(
            Deg(fov), 
            aspect, 
            near, 
            far,
        );
        projection[1][1] = projection[1][1] * -1.0;
        let eye = Point3::new(2.0, 2.0, 2.0);
        let center = Point3::new(0.0, 0.0, 0.0);
        let up = Vector3::new(0.0, 0.0, 1.0);
        let view = Matrix4::look_at_rh(eye,center, up);
        return Self{
            matrix : UniformBuffer{matrix:projection*view},
            projection,
            view,
            near,
            far,
            fov,
            aspect,
            eye,
            center,
            up,
            left_mouse_button_pressed : false,
        }
    }
    ///Ensure that the aspect ratio does not change the width of objects
    pub fn correct_perspective(&mut self, extent : Extent2D){
        self.aspect = extent.width as f32 / extent.height as f32;
        self.projection = cgmath::perspective(Deg(self.fov), self.aspect, self.near, self.far);
        self.projection[1][1] = self.projection[1][1] * -1.0;
    }
    ///Set the projview matrix
    pub fn update(&mut self){
        self.matrix.matrix=self.projection*self.view;
    }
    ///Register mouse movement and update the camera
    pub fn mouse_movement(&mut self, delta : (f64,f64)){
        if self.left_mouse_button_pressed{
            let sin_x = cgmath::Angle::sin(Deg(delta.0 as f32/2.0));
            let cos_x = cgmath::Angle::cos(Deg(delta.0 as f32/2.0));
            let quat_alpha = Quaternion::new(0.0, self.up.x - self.center.x, self.up.y - self.center.y, self.up.z - self.center.z).normalize();
            let mut quat_q = sin_x * quat_alpha;
            quat_q.s += cos_x;
            let quat_beta = Quaternion::new(0.0, self.eye.x-self.center.x, self.eye.y-self.center.y, self.eye.z-self.center.z);
            let new_eye = (quat_q*quat_beta*quat_q.conjugate()).v;
            self.eye = Point3::new(new_eye.x+self.center.x, new_eye.y+self.center.y, new_eye.z+self.center.z);
            self.view = Matrix4::look_at_rh(self.eye,self.center, self.up);
        }
    }
    ///Zoom in or out
    pub fn mouse_zoom(&mut self, delta : f32){
        if self.fov + delta/100.0 < 180.0 && self.fov + delta/100.0 > 0.0{
            self.fov += delta/100.0;
            self.projection = cgmath::perspective(Deg(self.fov), self.aspect, self.near, self.far); 
            self.projection[1][1] = self.projection[1][1] * -1.0;
        };
    }
}