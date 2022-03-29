#![feature(box_syntax)]

use std::vec;

use renderer::{Renderer, RenderResult};

//Physical constants
const GRIDELEMENTSCALE: f32 = 0.05;//The size of a grid element in meters(denoted in equations as delta x)
const TIMESTEPSIZE: f32 = 0.0005;//The size of a time step size in seconds
const DENSITY: f32 = 997.0;//Density of the liquid in kg/m^{3}. We simulate water.
const EXTERNALFORCE : [f32; 3] = [0.0,0.0, 0.0];//Gravity in N
const VISCOSITY: f32 = 0.001;//Viscosity in Pa*s.
const ATMOSPHERIC_PRESSURE: f32=101325.0;//101 325;//Atmospheric pressure in Pa
const MAXITERATIONSPERTIMEFRAME:i32=2000;//This constant sets a maximum so the computer can not get in an infinite loop.
const RELEXATION: f32=1.0;//Pressure correction is often underestimated, this factor should be between 1.4 and 1.8.
const ALLOWEDERROR: f32=0.005; 

//Pressure is measured in Pascal, because it is the standard SI unit for pressure.

//Grid size(e.g. number of elements in each dimension)
const PRESSUREGRIDSIZE: [usize; 3] = [10,10,10];//x,y,z

pub struct VelocityGrid{
    grid: Vec<Vec<Vec<f32>>>,
    dimension: usize,
}

pub fn initialize_simulation(){
    let renderer = Renderer::new(false);
    let mut pressure_grid: Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>= box[[[0.0; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]];//pressureGrid[x][y][z] is the pressure at coordinates (x,y,z)
    let mut velocity_x = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+1], dimension:0};// z,y,x !!!
    let mut velocity_y = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+1]; PRESSUREGRIDSIZE[0]+2], dimension:1}; 
    let mut velocity_z = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+1]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+2], dimension:2}; 
    
    initialize_pressure_grid(&mut pressure_grid);
    let mut i: i32=0;
    loop{
    //for i in 0..500{
        let render_data = simulation_time_step(&mut velocity_x, &mut velocity_y, &mut velocity_z, &mut pressure_grid, i);
        renderer.transform_grid(render_data);
        match renderer.await_request(){
          RenderResult::NextStep => {}
           RenderResult::Shutdown=>{return}
        };
        i=i+1;
    }
    println!("Simulation finished");
}
fn initialize_pressure_grid(pressure_grid: &mut [[[f32; PRESSUREGRIDSIZE[2]];PRESSUREGRIDSIZE[1]];PRESSUREGRIDSIZE[0]]){
    for x in 0..(PRESSUREGRIDSIZE[0]-1){//velocty_grid has PRESSUREGRIDSIZE[dimension] elements, so loop from 0 to PRESSUREGRIDSIZE[dimension]-1.
        for y in 0..(PRESSUREGRIDSIZE[1]-1){
            for z in 0..(PRESSUREGRIDSIZE[2]-1){
                //The pressure should be the atmosferic pressure(101,325Pa) plus the pressure that is exercised by the water above a point on the water at that point. 
                pressure_grid[x][y][z]=ATMOSPHERIC_PRESSURE-DENSITY*(PRESSUREGRIDSIZE[2] as f32 - z as f32)*GRIDELEMENTSCALE*EXTERNALFORCE[2];
            }
        }
    }
}

fn simulation_time_step(velocity_grid_x: &mut Box<VelocityGrid>, velocity_grid_y: &mut Box<VelocityGrid>, velocity_grid_z: &mut Box<VelocityGrid>,  pressure_grid: &mut Box<[[[f32; PRESSUREGRIDSIZE[2]];PRESSUREGRIDSIZE[1]];PRESSUREGRIDSIZE[0]]>, time_step: i32) -> Vec<Vec<Vec<([f32;3],[f32;3])>>>{
    let i:&mut i32=&mut 0;
    while *i<MAXITERATIONSPERTIMEFRAME {
        //let direction_has_changed=false;
        //1) Predict u, v and w,
        let mut provisional_velocity_x = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+1], dimension:0};
        let mut provisional_velocity_y = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+1]; PRESSUREGRIDSIZE[0]+2], dimension:1}; 
        let mut provisional_velocity_z = box VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+1]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+2], dimension:2}; 
    
        //x-velocity
        predict_velocity(&mut provisional_velocity_x, &velocity_grid_x, &velocity_grid_y, &velocity_grid_z, pressure_grid);
        //y-velocity
        predict_velocity(&mut provisional_velocity_y, &velocity_grid_y, &velocity_grid_x, &velocity_grid_z, pressure_grid);
        //z-velocity
        predict_velocity(&mut provisional_velocity_z, &velocity_grid_z, &velocity_grid_x, &velocity_grid_y, pressure_grid);
        
        //2)Update boundary conditions(i.e. set walls)
        set_wall_boundary_conditions( &mut provisional_velocity_x,  &mut provisional_velocity_y,  &mut provisional_velocity_z, 1.0, time_step);

        //3)Calculate pressure correction
        let pressure_correction: Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>=calculate_pressure_correction(&provisional_velocity_x, &provisional_velocity_y, &provisional_velocity_z);
        println!("Pressure correction at (1, 1, 1): {}, pressure correction at (6, 6, 2): {}", pressure_correction[1][1][1], pressure_correction[6][6][2]);
        println!("Pressure at (1, 1, 1): {}, pressure at (6, 6, 2): {}", pressure_grid[1][1][1], pressure_grid[6][6][2]);
        //4)Update u and v
        update_velocity_field(&mut provisional_velocity_x, &pressure_correction);
        update_velocity_field(&mut provisional_velocity_y, &pressure_correction);
        update_velocity_field(&mut provisional_velocity_z, &pressure_correction);
        
        //5)Update boundary values
        set_wall_boundary_conditions( &mut provisional_velocity_x,  &mut provisional_velocity_y,  &mut provisional_velocity_z, 1.0, time_step);
        
        //6)Check convergence
        if check_convergence(&provisional_velocity_x, &provisional_velocity_y, &provisional_velocity_z) {// If the continuity equation has converged we can go to the next timestep
            velocity_grid_x.grid=provisional_velocity_x.grid.clone();
            velocity_grid_y.grid=provisional_velocity_y.grid.clone();
            velocity_grid_z.grid=provisional_velocity_z.grid.clone();
            println!("Finished in {} steps, inflow is {}", i,some_sigmoid_function(time_step));
            println!("Pressure at (8,8,1) is {} on timestep {}", pressure_grid[8][8][1], time_step);    
            *i=MAXITERATIONSPERTIMEFRAME;
        }else{
            //println!{"convergence has not yet been reached, trying again, iteration: {}, timestep {}", i, time_step};
            //println!{"Pressure {} correction {} at (8,8,1)", pressure_grid[8][8][1], pressure_correction[8][8][1]};
            if *i+1==MAXITERATIONSPERTIMEFRAME{//If the continuity equation has not converged after many iterations something probably went wrong. Therefore the program will have to be terminated then.
                println!("Last iteration {} did not converge", i);
                std::process::exit(1);
            }
            
        }
        *i=*i+1;
        //println!("i is {}", i);
        //7) Update pressure
        update_pressure(pressure_grid, &pressure_correction);
    }  
    return convert_velocities_to_collocated_grid_and_visualise([0,4,0], [PRESSUREGRIDSIZE[0]-1, 4, PRESSUREGRIDSIZE[2]-1], [9,1,9], velocity_grid_x, velocity_grid_y, velocity_grid_z);
}

//min_coords and max_coords are the pressure coordinates of which we want to know the velocities(this function will determine those velocities by taking the average of nearby velocities)
//data_grid_point_size is the size of the grid we want to show to the user
pub fn convert_velocities_to_collocated_grid_and_visualise(min_coords: [usize; 3], max_coords: [usize;3], data_grid_point_size: [usize; 3], velocity_grid_x: &VelocityGrid, velocity_grid_y: &VelocityGrid, velocity_grid_z: &VelocityGrid) -> Vec<Vec<Vec<([f32;3],[f32;3])>>>{
    let step_size=[calc_step_size(max_coords[0]-min_coords[0], data_grid_point_size[0]), calc_step_size(max_coords[1]-min_coords[1], data_grid_point_size[1]), calc_step_size(max_coords[2]-min_coords[2], data_grid_point_size[2])];
    let mut return_data: Vec<Vec<Vec<([f32; 3],[f32;3])>>>=vec![vec![vec![([0.0; 3],[0.0,0.0,0.0]); data_grid_point_size[0]]; data_grid_point_size[1]]; data_grid_point_size[0]];
    for x in 0..data_grid_point_size[0]{
        for y in 0..data_grid_point_size[1]{
            for z in 0..data_grid_point_size[2]{
                return_data[x][y][z]=([get_velocity_at_pressure_point(&velocity_grid_x, x*step_size[0], y*step_size[1], z*step_size[2]),  get_velocity_at_pressure_point(&velocity_grid_y, x*step_size[0], y*step_size[1], z*step_size[2]), get_velocity_at_pressure_point(&velocity_grid_z, x*step_size[0], y*step_size[1], z*step_size[2])], [0.0,0.0,0.0]);
            }
        }
    }
    return return_data;
}


//Calculates the interval between the velocities that should be shown in one dimension
fn calc_step_size(from_dimension: usize, to_dimension: usize)->usize{
    return from_dimension/to_dimension;
} 


fn predict_velocity(provisonal_velocity_field: &mut VelocityGrid, velocity_field_last_time_step: &VelocityGrid, orthogonal_velocity_field_a: &VelocityGrid, orthogonal_velocity_field_b: &VelocityGrid, pressure_grid: &Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>){
    let dim=get_dimension(provisonal_velocity_field.dimension);
    for x in 1..(PRESSUREGRIDSIZE[0]-dim[0]+1) {
        for y in 1..(PRESSUREGRIDSIZE[1]-dim[1]+1) {
            for z in 1..(PRESSUREGRIDSIZE[2]-dim[2]+1) {
                //Diffusion term
                let diffusion=VISCOSITY*(laplacian(velocity_field_last_time_step, x, y, z));
                //And finally, the provisional velocity
                provisonal_velocity_field.grid[x][y][z]=velocity_field_last_time_step.grid[x][y][z]+TIMESTEPSIZE/DENSITY*(-convection_term(velocity_field_last_time_step, orthogonal_velocity_field_a, orthogonal_velocity_field_b, x, y, z)-first_order_central_spatial_pressure_derivative(&pressure_grid, x-1, y-1, z-1, velocity_field_last_time_step.dimension)+diffusion+DENSITY*EXTERNALFORCE[velocity_field_last_time_step.dimension]);
            }
        }
    }
}

fn calculate_pressure_correction(x_velocity: & VelocityGrid, y_velocity: & VelocityGrid, z_velocity: & VelocityGrid)->Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>{
    let mut pressure_correction: Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>=box [[[0.0; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]];//Here we will store the pressure corrections.
    let constant_term_pressure_equation=RELEXATION*DENSITY*GRIDELEMENTSCALE/(6.0*TIMESTEPSIZE);//The lower part of the equation is this constant.
        for i in 0..PRESSUREGRIDSIZE[0] - 1{
            for j in 0..PRESSUREGRIDSIZE[1] - 1{
                for k in 0..PRESSUREGRIDSIZE[2] - 1{
                    pressure_correction[i][j][k]=-constant_term_pressure_equation*(x_velocity.grid[i+1][j+1][k+1] - x_velocity.grid[i][j+1][k+1]+ y_velocity.grid[i+1][j+1][k+1] - y_velocity.grid[i+1][j][k+1] + z_velocity.grid[i+1][j+1][k+1]-z_velocity.grid[i+1][j+1][k]);
                }
            }
        }
    return pressure_correction;
}

fn convection_term(velocity_field_last_time_step: &VelocityGrid,orthogonal_velocity_field_a: &VelocityGrid, orthogonal_velocity_field_b: &VelocityGrid, x: usize, y:usize, z:usize ) -> f32{// calculate the convection term
     return DENSITY*(velocity_field_last_time_step.grid[x][y][z]*second_order_spatial_derivative(&velocity_field_last_time_step, x, y, z, velocity_field_last_time_step.dimension)
                +get_velocity_from_orthogonal_grid(&orthogonal_velocity_field_a, x, y, z, velocity_field_last_time_step.dimension)*second_order_spatial_derivative(velocity_field_last_time_step, x, y, z, orthogonal_velocity_field_a.dimension)
                +get_velocity_from_orthogonal_grid(&orthogonal_velocity_field_b, x, y, z, velocity_field_last_time_step.dimension)*second_order_spatial_derivative(velocity_field_last_time_step, x, y, z, orthogonal_velocity_field_b.dimension));
}

fn update_velocity_field(velocity_field: &mut VelocityGrid, pressure_correction : &[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]){
    let dim=get_dimension(velocity_field.dimension);
    let constant_term_velocity_equation=TIMESTEPSIZE/(DENSITY*GRIDELEMENTSCALE);
    for i in 1..PRESSUREGRIDSIZE[0]+1-dim[0]{
        for j in 1..PRESSUREGRIDSIZE[1]+1-dim[1]{
            for k in 1..PRESSUREGRIDSIZE[2]+1-dim[2]{
                velocity_field.grid[i][j][k]=velocity_field.grid[i][j][k]-constant_term_velocity_equation*(pressure_correction[i+dim[0]-1][j+dim[1]-1][k+dim[2]-1]- pressure_correction[i-1][j-1][k-1]);
            }
        }
    }
}

fn update_pressure(pressure_grid: &mut [[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]], pressure_correction: &[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]){
    for i in 0..PRESSUREGRIDSIZE[0]{
        for j in 0..PRESSUREGRIDSIZE[1]{
            for k in 0..PRESSUREGRIDSIZE[2]{
                pressure_grid[i][j][k]=pressure_grid[i][j][k]+pressure_correction[i][j][k];
            }
        }
    }
}

fn check_convergence_at_point(provisional_velocity_x: &VelocityGrid, provisional_velocity_y: &VelocityGrid, provisional_velocity_z: &VelocityGrid, x:usize, y:usize, z:usize)->f32{
    return first_order_central_spatial_derivative_at_pressure_coordinates(&provisional_velocity_x, x, y, z)
    +first_order_central_spatial_derivative_at_pressure_coordinates(&provisional_velocity_y, x, y, z)
    +first_order_central_spatial_derivative_at_pressure_coordinates(&provisional_velocity_z, x, y, z);
}

fn check_convergence(provisional_velocity_x:&VelocityGrid, provisional_velocity_y: &VelocityGrid, provisional_velocity_z: &VelocityGrid)->bool{
    
    for x in 1..PRESSUREGRIDSIZE[0]-1{
        for y in 1..PRESSUREGRIDSIZE[1]-1{
            for z in 1..PRESSUREGRIDSIZE[2]-1{
                let error=check_convergence_at_point(provisional_velocity_x, provisional_velocity_y, provisional_velocity_z, x, y, z);   
                if error.abs()>ALLOWEDERROR{
                    //println!("Convergence not yet reached, error is {} at ({}, {}, {})", error, x, y, z );
                    return false;

                }
            }
        }
    } 
    return true;
}

//Set the wall boundary conditions
fn set_wall_boundary_conditions(velocity_grid_x: &mut VelocityGrid, velocity_grid_y: &mut VelocityGrid, velocity_grid_z: &mut VelocityGrid, x_wall_velocity: f32, time_step:i32){
    set_boundary_conditions_of_two_parallel_walls(velocity_grid_x, velocity_grid_y, velocity_grid_z, 0.0);
    set_boundary_conditions_of_two_parallel_walls(velocity_grid_y, velocity_grid_x, velocity_grid_z, 0.0);
    set_boundary_conditions_of_two_parallel_walls(velocity_grid_z, velocity_grid_x, velocity_grid_y, 0.0);
    create_inflow_or_outflow(velocity_grid_x, velocity_grid_y, velocity_grid_z, [0,2,2], [0,6,6], -some_sigmoid_function(time_step));
    create_inflow_or_outflow(velocity_grid_z, velocity_grid_y, velocity_grid_x, [2,2,0], [6,6,0], some_sigmoid_function(time_step));
} 

fn some_sigmoid_function(time_step: i32)->f32{
    let t=time_step as f32;
    return 0.1;//1.0/(f32::powf(2.7182818, 3.0-t)+1.0);
}

fn set_boundary_conditions_of_two_parallel_walls(orthogonal_velocity_grid: &mut VelocityGrid, parallel_velocity_grid_a: &mut VelocityGrid, parallel_velocity_grid_b: &mut VelocityGrid, orthogonal_velocity_grid_value: f32){
    let dim= get_dimension(orthogonal_velocity_grid.dimension);
    //Set the max positions, the position coordinate orthogonal to the wall will be set to zero later
    let mut max_orthogonal_coords=[PRESSUREGRIDSIZE[0]+1, PRESSUREGRIDSIZE[1]+1, PRESSUREGRIDSIZE[2]+1];//max coordinates for orthogonal velocities  
    let mut max_parallel_coords=PRESSUREGRIDSIZE;//Max coordinates for parallel velocities
    //The coordinates of one wall have coordinate zero in one dimension
    max_orthogonal_coords[orthogonal_velocity_grid.dimension]=0;//Take the wall that has the 0 coordinate in one direction
    max_parallel_coords[orthogonal_velocity_grid.dimension]=0;// The sizes of the parallel grids are the same in the other dimensions, so we will loop through the same values.
    //Set boundary conditions for the zero wall
    set_orthogonal_boundary_condition_at_wall(orthogonal_velocity_grid, [0,0,0], max_orthogonal_coords, orthogonal_velocity_grid_value);
    set_parallel_boundary_condition_at_wall(parallel_velocity_grid_a, [0,0,0], max_parallel_coords, false, orthogonal_velocity_grid.dimension);
    set_parallel_boundary_condition_at_wall(parallel_velocity_grid_b, [0,0,0], max_parallel_coords, false, orthogonal_velocity_grid.dimension);
    //The other wall has one coordinate at the maximum, so set that coordinate to the maximum
    max_orthogonal_coords[orthogonal_velocity_grid.dimension]=PRESSUREGRIDSIZE[orthogonal_velocity_grid.dimension];
    max_parallel_coords[orthogonal_velocity_grid.dimension]=PRESSUREGRIDSIZE[orthogonal_velocity_grid.dimension]+1;
    let minimum_parallel_coords=[(PRESSUREGRIDSIZE[0]+1)*dim[0], (PRESSUREGRIDSIZE[1]+1)*dim[1], (PRESSUREGRIDSIZE[2]+1)*dim[2]];
    let minimum_orthogonal_coords=[PRESSUREGRIDSIZE[0]*dim[0],PRESSUREGRIDSIZE[1]*dim[1], PRESSUREGRIDSIZE[2]*dim[2]];
    //Set boundary conditions for the maximum wall
    set_orthogonal_boundary_condition_at_wall(orthogonal_velocity_grid, minimum_orthogonal_coords, max_orthogonal_coords, orthogonal_velocity_grid_value);
    set_parallel_boundary_condition_at_wall(parallel_velocity_grid_a, minimum_parallel_coords, max_parallel_coords, true, orthogonal_velocity_grid.dimension);
    set_parallel_boundary_condition_at_wall(parallel_velocity_grid_b, minimum_parallel_coords, max_parallel_coords, true, orthogonal_velocity_grid.dimension);
    

}


//Set the orthogonal velocity to a certain value on a wall
fn set_orthogonal_boundary_condition_at_wall(orthogonal_velocity_grid: &mut VelocityGrid, min_coords: [usize; 3], max_coords: [usize; 3], value: f32){
    for x in min_coords[0]..=max_coords[0]{
        for y in min_coords[1]..=max_coords[1]{
            for z in min_coords[2]..=max_coords[2]{
                orthogonal_velocity_grid.grid[x][y][z]=value;
            }
        }
    }
}

fn create_inflow_or_outflow(orthogonal_velocity_grid: &mut VelocityGrid, parallel_velocity_grid_a: &mut VelocityGrid, parallel_velocity_grid_b: &mut VelocityGrid, min_orthogonal_coords: [usize;3], max_orthogonal_coords: [usize; 3], flow: f32){
    for x in min_orthogonal_coords[0]..=max_orthogonal_coords[0]{
        for y in min_orthogonal_coords[1]..=max_orthogonal_coords[1]{
            for z in min_orthogonal_coords[2]..=max_orthogonal_coords[2]{
                orthogonal_velocity_grid.grid[x][y][z]=flow;
            }
        }
    }
    //For the parallel velocities the size should be one larger in all dimensions
    for x in min_orthogonal_coords[0]..=max_orthogonal_coords[0]+1{
        for y in min_orthogonal_coords[1]..=max_orthogonal_coords[1]+1{
            for z in min_orthogonal_coords[2]..=max_orthogonal_coords[2]+1{
                parallel_velocity_grid_a.grid[x][y][z]=0.0;
                parallel_velocity_grid_b.grid[x][y][z]=0.0;
            }
        }
    }
    
}

//wall_is_on_lower_side=0 means the wall is on the side with lower coordinates seen from the dry side and wall_is_on_lower_side=1 means the wall is on the side with higher coordinates. 
//orthogonal_dimension is the dimension number(0 for x, 1 for y, 2 for z) of the dimension orthogonal to the wall
fn set_parallel_boundary_condition_at_wall(parallel_velocity_grid: &mut VelocityGrid, min_coords: [usize; 3], max_coords: [usize; 3], wall_is_on_lower_side: bool, orthogonal_dimension: usize){
    let dim=get_dimension(orthogonal_dimension);
    let transformation_in_one_dimension=1 - 2 * (wall_is_on_lower_side as isize);// -1 when a lower element is needed, +1 when a higher element is needed
    let transformation_to_neighbor:[isize; 3]=[(dim[0] as isize) * transformation_in_one_dimension, (dim[1] as isize) * transformation_in_one_dimension, (dim[2] as isize) * transformation_in_one_dimension];// This is the transformation to the neighbor opposite of the wall
    for x in min_coords[0]..=max_coords[0]{
        for y in min_coords[1]..=max_coords[1]{
            for z in min_coords[2]..=max_coords[2]{
                //The parallel velocity should be the opposite of the parallel velocity on the other side of the wall, so that the average is zero.
                parallel_velocity_grid.grid[x][y][z]=-parallel_velocity_grid.grid[(x as isize + transformation_to_neighbor[0])as usize][(y as isize + transformation_to_neighbor[1]) as usize][(z as isize+transformation_to_neighbor[2]) as usize];          
            }
        }
    }
}

fn first_order_central_spatial_pressure_derivative(f: &Box<[[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]>, x:usize, y:usize, z:usize, dimension_number:usize) -> f32{
    let position_difference=get_dimension(dimension_number);
    return (f[x+position_difference[0]][y+position_difference[1]][z+position_difference[2]]-f[x][y][z])/GRIDELEMENTSCALE;
}

fn first_order_forward_spatial_derivative(f: &VelocityGrid, x:usize, y:usize, z:usize) -> f32{
    let position_difference=get_dimension(f.dimension);
    return (f.grid[x+position_difference[0]][y+position_difference[1]][z+position_difference[2]]-f.grid[x][y][z])/GRIDELEMENTSCALE;
}

fn first_order_central_spatial_derivative_at_pressure_coordinates(f: &VelocityGrid, x: usize, y: usize, z:usize)->f32{//Calculates the central spatial derivative, uses pressure coordinates
    let dim=get_dimension(f.dimension);
    return (f.grid[x+1][y+1][z+1]-f.grid[x+1-dim[0]][y+1-dim[1]][z+1-dim[2]])/GRIDELEMENTSCALE;
}

fn second_order_spatial_derivative(f:&VelocityGrid, x: usize, y:usize, z:usize, dimension_number:usize) -> f32{
    let dim= get_dimension(dimension_number);
    return (f.grid[x+dim[0]][y+dim[1]][z+dim[2]] - f.grid[x-dim[0]][y-dim[1]][z-dim[2]])/(2.0*GRIDELEMENTSCALE);
}

fn second_order_second_spatial_derivative(f: &VelocityGrid, x:usize, y:usize, z:usize, dimension_number:usize) -> f32{
    let dim = get_dimension(dimension_number);
    return (f.grid[x+dim[0]][y+dim[1]][z+dim[2]]-2.0*f.grid[x][y][z]+f.grid[x-dim[0]][y-dim[1]][z-dim[2]])/(GRIDELEMENTSCALE*GRIDELEMENTSCALE);
}

//Laplacian velocity grid
fn laplacian(f: &VelocityGrid, x:usize, y:usize, z:usize)->f32{
    return second_order_second_spatial_derivative(f, x, y, z, 0)+second_order_second_spatial_derivative(f, x, y, z, 1)+second_order_second_spatial_derivative(f, x, y, z, 2);
}

//This function will retrieve the velocity of an orthogonal grid a grid point of another grid.
fn get_velocity_from_orthogonal_grid(orthogonal_grid: &VelocityGrid, x:usize, y:usize, z:usize, other_grid_dimension:usize) -> f32{
    let dim_to=get_dimension(other_grid_dimension);
    let dim_from=get_dimension(orthogonal_grid.dimension);
    return 0.25*(orthogonal_grid.grid[x-dim_from[0]][y-dim_from[1]][z-dim_from[2]]//Left down
        +orthogonal_grid.grid[x-dim_from[0]+dim_to[0]][y-dim_from[1]+dim_to[1]][z-dim_from[2]+dim_to[2]]//left up
        +orthogonal_grid.grid[x][y][z]//right down
        +orthogonal_grid.grid[x+dim_to[0]][y+dim_to[1]][z+dim_to[2]]);//right up
}

fn get_velocity_at_pressure_point(velocity_grid: &VelocityGrid, x: usize, y: usize, z: usize)->f32{
    let dim= get_dimension(velocity_grid.dimension);
    return velocity_grid.grid[x+1][y+1][x+1]-velocity_grid.grid[x+1-dim[0]][y+1-dim[1]][z+1-dim[2]];//Just take the average
}

//Gives you the unit vector of the dimension with the given numer.
//x - 0, y - 1, z - 2
fn get_dimension(dimension_number:usize)->[usize; 3]{
    let mut dim = [0,0,0];
    dim[dimension_number]=1;
    return dim;
}
