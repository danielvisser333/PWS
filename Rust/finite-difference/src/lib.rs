use std::vec;

use renderer::Renderer;

//Physical constants
const GRIDELEMENTSCALE: f32 = 0.05;//The size of a grid element in meters
const TIMESTEPSIZE: f32 = 0.1;//The size of a time step size in seconds
const DENSITY: f32 = 1000.0;//Density of the liquid in kg/m^{3}. We simulate water.
const EXTERNALFORCE : [f32; 3] = [0.0,0.0,-9.81];//Gravity in N
const VISCOSITY: f32 = 0.001;//Viscosity in Pa*s.

const MAXITERATIONSPERTIMEFRAME:i32=50;
const RELEXATION: f32=1.0;//Pressure correction is often underestimated, this factor should be between 1.4 and 1.8.
const ALLOWEDERROR: f32=0.00001; 

//Pressure is measured in Pascal, because it is the standard SI unit for pressure.

//Grid size(e.g. number of elements in each dimension)
const PRESSUREGRIDSIZE: [usize; 3] = [10,10,10];//x,y,z

pub struct VelocityGrid{
    grid: Vec<Vec<Vec<f32>>>,
    dimension: usize,
}

pub fn initialize_simulation(){
    let renderer = Renderer::new(false);
    let mut pressure_grid: [[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]=[[[0.0; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]];//pressureGrid[x][y][z] is the pressure at coordinates (x,y,z)
    //The size of the velocity grid should be 1 bigger in every spatial dimension(so that's all dimensions in the array) because we use a collocated grid
    let mut velocity_x = VelocityGrid{grid: vec![vec![vec![1.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+1], dimension:0};
    let mut velocity_y = VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+1]; PRESSUREGRIDSIZE[0]+2], dimension:1}; 
    let mut velocity_z = VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+1]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+2], dimension:2}; 
    //Store the 
    let mut boundary_points: Vec<[f32; 3]> = Vec::new();
    
    initialize_pressure_grid(&mut pressure_grid);

    let render_data = simulation_time_step(&mut velocity_x, &mut velocity_y, &mut velocity_z, &mut pressure_grid);
    renderer.transform_grid(render_data);

    renderer.await_close_request();
}

fn initialize_pressure_grid(pressure_grid: &mut [[[f32; PRESSUREGRIDSIZE[2]];PRESSUREGRIDSIZE[1]];PRESSUREGRIDSIZE[0]]){
    for x in 0..(PRESSUREGRIDSIZE[0]-1){//velocty_grid has PRESSUREGRIDSIZE[dimension] elements, so loop from 0 to PRESSUREGRIDSIZE[dimension]-1.
        for y in 0..(PRESSUREGRIDSIZE[1]-1){
            for z in 0..(PRESSUREGRIDSIZE[2]-1){
                //The pressure should be the atmosferic pressure(101,325Pa) plus the pressure that is exercised by the water above a point on the water at that point. 
                pressure_grid[x][y][z]=101.325+9810.0*(PRESSUREGRIDSIZE[2] as f32 - z as f32 -1.0)*GRIDELEMENTSCALE;
            }
        }
    }
}

fn simulation_time_step(velocity_grid_x: &mut VelocityGrid, velocity_grid_y: &mut VelocityGrid, velocity_grid_z: &mut VelocityGrid,  pressure_grid: &mut [[[f32; PRESSUREGRIDSIZE[2]];PRESSUREGRIDSIZE[1]];PRESSUREGRIDSIZE[0]]) -> Vec<Vec<Vec<[f32;3]>>>{
    let i:&mut i32=&mut 0;
    while *i<MAXITERATIONSPERTIMEFRAME {
        //1) Predict u, v and w,
        let mut provisional_velocity_x = VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+1], dimension:0};
        let mut provisional_velocity_y = VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+2]; PRESSUREGRIDSIZE[1]+1]; PRESSUREGRIDSIZE[0]+2], dimension:1}; 
        let mut provisional_velocity_z = VelocityGrid{grid: vec![vec![vec![0.0;PRESSUREGRIDSIZE[2]+1]; PRESSUREGRIDSIZE[1]+2]; PRESSUREGRIDSIZE[0]+2], dimension:2}; 
    
        //x-velocity
        predict_velocity(&mut provisional_velocity_x, &velocity_grid_x, &velocity_grid_y, &velocity_grid_z, *pressure_grid);
        //y-velocity
        predict_velocity(&mut provisional_velocity_y, &velocity_grid_y, &velocity_grid_x, &velocity_grid_z, *pressure_grid);
        //z-velocity
        predict_velocity(&mut provisional_velocity_z, &velocity_grid_z, &velocity_grid_x, &velocity_grid_y, *pressure_grid);
        
        //2)Update boundary conditions(i.e. set walls)
        set_wall_boundary_conditions( &mut velocity_grid_x.grid,  &mut velocity_grid_y.grid,  &mut velocity_grid_z.grid);

        //3)Calculate pressure
        //NOTE: When convergence is not reached velocity data from last timestep will be used for the next iteration. However, pressure data from this iteration will be used. Therefore, store the pressure data permenantly.
        let lowerpartPressureEquation=6.0*TIMESTEPSIZE/DENSITY*f32::powf(GRIDELEMENTSCALE, -2.0);//The lower part of the equation is this constant.
        for x in 0..PRESSUREGRIDSIZE[0] - 1{
            for y in 0..PRESSUREGRIDSIZE[1] - 1{
                for z in 0..PRESSUREGRIDSIZE[2] - 1{
                    pressure_grid[x][y][z]=pressure_grid[x][y][z]-RELEXATION*(first_order_forward_spatial_derivative(&provisional_velocity_x, x, y+1, z+1)
                    +first_order_forward_spatial_derivative(&provisional_velocity_y, x+1, y, z+1)
                    +first_order_forward_spatial_derivative(&provisional_velocity_z, x+1, y+1, z))/lowerpartPressureEquation;
                }
            }
        }
        //4)Update u and v
        update_velocity_field(&mut provisional_velocity_x);
        update_velocity_field(&mut provisional_velocity_y);
        update_velocity_field(&mut provisional_velocity_z);
        
        //5)Update boundary values
        crate::set_wall_boundary_conditions( &mut provisional_velocity_x.grid,  &mut provisional_velocity_y.grid,  &mut provisional_velocity_z.grid);
        
        //6)Check convergence
        let mut total_error:f32=0.0;
        for x in 0..PRESSUREGRIDSIZE[0] - 1{
            for y in 0..PRESSUREGRIDSIZE[1] - 1{
                for z in 0..PRESSUREGRIDSIZE[2] - 1{
                    let error=first_order_central_spatial_derivative(&provisional_velocity_x, x, y, z)
                        +first_order_central_spatial_derivative(&provisional_velocity_y, x, y, z)
                        +first_order_central_spatial_derivative(&provisional_velocity_z, x, y, z);    
                    if error.abs()<ALLOWEDERROR{
                        total_error=total_error+error;

                    }
                }
            }
        } 
        if(total_error.abs()<ALLOWEDERROR){
            velocity_grid_x.grid=provisional_velocity_x.grid.clone();
            velocity_grid_y.grid=provisional_velocity_y.grid.clone();
            velocity_grid_z.grid=provisional_velocity_z.grid.clone();
            *i=MAXITERATIONSPERTIMEFRAME;
            println!("Converged! total errrow is {}", total_error);
        }else{
            println!("Iteration {} did not converge, total eroor was: {}, max error was {} ", i, total_error, ALLOWEDERROR);
        }
        *i=*i+1;
        println!("i is {}", i);
    }  
    println!("Finished! At (2,2,2) velocity is ({}, {}, {})",velocity_grid_x.grid[2][2][2], velocity_grid_y.grid[2][2][2],velocity_grid_z.grid[2][2][2]);
    return convert_velocities_to_collocated_grid_and_visualise([0,0,0], PRESSUREGRIDSIZE, [5,5,5], velocity_grid_x, velocity_grid_y, velocity_grid_z);
}

pub fn convert_velocities_to_collocated_grid_and_visualise(min_coords: [usize; 3], max_coords: [usize;3], data_grid_point_size: [usize; 3], velocity_grid_x: &VelocityGrid, velocity_grid_y: &VelocityGrid, velocity_grid_z: &VelocityGrid) -> Vec<Vec<Vec<[f32;3]>>>{
    let mut step_size=[calc_step_size(max_coords[0]-min_coords[0], data_grid_point_size[0]), calc_step_size(max_coords[1]-min_coords[1], data_grid_point_size[1]), calc_step_size(max_coords[2]-min_coords[2], data_grid_point_size[2])];
    let mut return_data: Vec<Vec<Vec<[f32; 3]>>>=vec![vec![vec![[0.0; 3]; data_grid_point_size[0]]; data_grid_point_size[1]]; data_grid_point_size[0]];
    for x in 0..data_grid_point_size[0]{
        for y in 0..data_grid_point_size[1]{
            for z in 0..data_grid_point_size[2]{
                return_data[x][y][z]=[get_velocity_at_pressure_point(&velocity_grid_x, x, y, z),  get_velocity_at_pressure_point(&velocity_grid_y, x, y, z), get_velocity_at_pressure_point(&velocity_grid_z, x, y, z)];
            }
        }
    }
    //CALL VISUALISATION FUNCTION HERE
    return return_data;
}
fn convert_coordinate_to_other_grid(min_coords : [usize; 3], step_size : [usize; 3], steps: usize, dim_number: usize) -> usize{
    return min_coords[dim_number]+step_size[dim_number]*steps;
}


fn calc_step_size(from_dimension: usize, to_dimension: usize)->usize{
    return from_dimension/to_dimension;
}

fn set_wall_boundary_conditions(provisional_velocity_x: &mut Vec<Vec<Vec<f32>>>, provisional_velocity_y: &mut Vec<Vec<Vec<f32>>>, provisional_velocity_z: &mut Vec<Vec<Vec<f32>>>){
    //min x wall
    set_boundary_condition_in_area(provisional_velocity_x,  provisional_velocity_y,  provisional_velocity_z, [0, 0, 0], [0, PRESSUREGRIDSIZE[1]+1, PRESSUREGRIDSIZE[2]+1], [1,0,0]);
    //max x wall
    set_boundary_condition_in_area(provisional_velocity_x,  provisional_velocity_y,  provisional_velocity_z, [PRESSUREGRIDSIZE[0], 0, 0], [PRESSUREGRIDSIZE[0], PRESSUREGRIDSIZE[1]+1, PRESSUREGRIDSIZE[2]+1], [-1,0,0]);
    //min y wall
    set_boundary_condition_in_area(provisional_velocity_y, provisional_velocity_x,  provisional_velocity_z ,[0, 0, 0], [PRESSUREGRIDSIZE[0]+1, 0, PRESSUREGRIDSIZE[2]+1], [0, 1, 0]);
    //max y wall
    set_boundary_condition_in_area(provisional_velocity_y,  provisional_velocity_x,  provisional_velocity_z, [0, PRESSUREGRIDSIZE[1], 0], [0, PRESSUREGRIDSIZE[1], PRESSUREGRIDSIZE[2]+1], [0, -1, 0]);
    //min z wal
    set_boundary_condition_in_area(provisional_velocity_z,  provisional_velocity_x,  provisional_velocity_y, [0, 0, 0], [PRESSUREGRIDSIZE[0]+1, PRESSUREGRIDSIZE[1]+1, 0], [0,0,1]);
    //max z wall
    set_boundary_condition_in_area(provisional_velocity_z,  provisional_velocity_x,  provisional_velocity_y, [0, 0, PRESSUREGRIDSIZE[2]], [PRESSUREGRIDSIZE[0]+1, PRESSUREGRIDSIZE[1]+1, PRESSUREGRIDSIZE[2]], [0,0,-1]);

}  


fn predict_velocity(provisonal_velocity_field: &mut VelocityGrid, velocity_field_last_time_step: &VelocityGrid, orthogonal_velocity_field_a: &VelocityGrid, orthogonal_velocity_field_b: &VelocityGrid, pressure_grid: [[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]]){
    let dim=get_dimension(provisonal_velocity_field.dimension);
    for x in 1..(PRESSUREGRIDSIZE[0]-dim[0]+1) {
        for y in 1..(PRESSUREGRIDSIZE[1]-dim[1]+1) {
            for z in 1..(PRESSUREGRIDSIZE[2]-dim[2]+1) {
                //convection term
                let convection=DENSITY*(velocity_field_last_time_step.grid[x][y][z]*second_order_spatial_derivative(&velocity_field_last_time_step, x, y, z, velocity_field_last_time_step.dimension)
                +get_velocity_from_orthogonal_grid(&orthogonal_velocity_field_a, x, y, z, velocity_field_last_time_step.dimension)*second_order_second_spatial_derivative(velocity_field_last_time_step, x, y, z, orthogonal_velocity_field_a.dimension)
                +get_velocity_from_orthogonal_grid(&orthogonal_velocity_field_b, x, y, z, velocity_field_last_time_step.dimension)*second_order_second_spatial_derivative(velocity_field_last_time_step, x, y, z, orthogonal_velocity_field_b.dimension));
                //Diffusion term
                let diffusion=VISCOSITY*(laplacian(velocity_field_last_time_step, x, y, z));
                //And finally, the provisional velocity
                provisonal_velocity_field.grid[x][y][z]=velocity_field_last_time_step.grid[x][y][z]+TIMESTEPSIZE/DENSITY*(-convection+first_order_central_spatial_pressure_derivative(pressure_grid, x-1, y-1, z-1, velocity_field_last_time_step.dimension)+diffusion+EXTERNALFORCE[velocity_field_last_time_step.dimension]);
            }
        }
    }

}

fn update_velocity_field(velocity_field: &mut VelocityGrid){
    let dim=get_dimension(velocity_field.dimension);
    let constant_term_velocity_equation=TIMESTEPSIZE/(DENSITY*GRIDELEMENTSCALE);
    for x in 1-dim[0]..PRESSUREGRIDSIZE[0]+1-dim[0]{
        for y in 1-dim[1]..PRESSUREGRIDSIZE[1]+1-dim[1]{
            for z in 1-dim[2]..PRESSUREGRIDSIZE[2]+1-dim[2]{
                velocity_field.grid[x][y][z]=velocity_field.grid[x][y][z]-constant_term_velocity_equation*first_order_forward_spatial_derivative(&velocity_field, x, y, z);
            }
        }
    }
}

fn set_boundary_condition_in_area(orthogonal_velocity_grid: &mut Vec<Vec<Vec<f32>>>, parallel_velocity_grid_a: &mut Vec<Vec<Vec<f32>>>, parallel_velocity_grid_b: &mut Vec<Vec<Vec<f32>>>, min_coords: [usize; 3], max_coords: [usize; 3], transformation_to_neighbor : [isize;3]){
    for x in min_coords[0]..max_coords[0]{
        for y in min_coords[1]..max_coords[1]{
            for z in min_coords[2]..max_coords[2]{
                orthogonal_velocity_grid[x][y][z]=0.0;
                let a=(x as isize + transformation_to_neighbor[0])as usize;
                parallel_velocity_grid_a[x][y][z]=-parallel_velocity_grid_a[(x as isize + transformation_to_neighbor[0])as usize][(y as isize + transformation_to_neighbor[1]) as usize][(z as isize+transformation_to_neighbor[2]) as usize];
                parallel_velocity_grid_b[x][y][z]=-parallel_velocity_grid_b[(x as isize + transformation_to_neighbor[0])as usize][(y as isize + transformation_to_neighbor[1]) as usize][(z as isize+transformation_to_neighbor[2]) as usize];
            
            }
        }
    }
}

fn first_order_central_spatial_pressure_derivative(f: [[[f32; PRESSUREGRIDSIZE[2]]; PRESSUREGRIDSIZE[1]]; PRESSUREGRIDSIZE[0]], x:usize, y:usize, z:usize, dimension_number:usize) -> f32{//TODO: generalize derivative functions for pressure and velocity
    let position_difference=get_dimension(dimension_number);
    return (f[x+position_difference[0]][y+position_difference[1]][z+position_difference[2]]-f[x][y][z])/GRIDELEMENTSCALE;
}

fn first_order_forward_spatial_derivative(f: &VelocityGrid, x:usize, y:usize, z:usize) -> f32{
    let position_difference=get_dimension(f.dimension);
    return (f.grid[x+position_difference[0]][y+position_difference[1]][z+position_difference[2]]-f.grid[x][y][z])/GRIDELEMENTSCALE;
}

fn first_order_central_spatial_derivative(f: &VelocityGrid, x: usize, y: usize, z:usize)->f32{//practically identical to first_order_forward_spatial_derivative, but for clarity I keep it.
    let dim=get_dimension(f.dimension);
    return (f.grid[x+dim[0]][y+dim[1]][z+dim[2]]-f.grid[x][y][z])/GRIDELEMENTSCALE;
}

fn second_order_spatial_derivative(f:&VelocityGrid, x: usize, y:usize, z:usize, dimension_number:usize) -> f32{
    let dim= get_dimension(dimension_number);
    return (f.grid[x+dim[0]][y+dim[1]][z+dim[2]] - f.grid[x-dim[0]][y-dim[1]][z-dim[2]])/(2.0*GRIDELEMENTSCALE);
}

fn second_order_second_spatial_derivative(f: &VelocityGrid, x:usize, y:usize, z:usize, dimension_number:usize) -> f32{
    let dim = get_dimension(dimension_number);
    return(f.grid[x+dim[0]][y+dim[1]][z+dim[2]]-2.0*f.grid[x][y][z]+f.grid[x-dim[0]][y-dim[1]][z-dim[2]])/(GRIDELEMENTSCALE*GRIDELEMENTSCALE);
}

//Laplacian velocity grid
fn laplacian(f: &VelocityGrid, x:usize, y:usize, z:usize)->f32{
    return second_order_second_spatial_derivative(f, x, y, z, 0)+second_order_second_spatial_derivative(f, x, y, z, 1)+second_order_second_spatial_derivative(f, x, y, z, 2);
}

//This function will retrieve the velocity of an orthogonal grid a gird point of another grid.
fn get_velocity_from_orthogonal_grid(orthogonal_grid: &VelocityGrid, x:usize, y:usize, z:usize, other_grid_dimension:usize) -> f32{
    let dim_from=get_dimension(other_grid_dimension);
    let dim_to=get_dimension(orthogonal_grid.dimension);
    return 0.25*(orthogonal_grid.grid[x-dim_to[0]][y-dim_to[1]][z-dim_to[2]]//Left down
        +orthogonal_grid.grid[x][y][z]//left up
        +orthogonal_grid.grid[x+dim_from[0]][y+dim_from[1]][z+dim_from[2]]//right up
        +orthogonal_grid.grid[x+dim_from[0]-dim_to[0]][y+dim_from[1]-dim_to[1]][z+dim_from[2]-dim_to[2]]);
}

fn get_velocity_at_pressure_point(velocity_grid: &VelocityGrid, x: usize, y: usize, z: usize)->f32{
    let dim= get_dimension(velocity_grid.dimension);
    return velocity_grid.grid[x+1][y+1][x+1]-velocity_grid.grid[x+1-dim[0]][y+1-dim[1]][z+1-dim[2]];
}

//Gives you the unit vector of the dimension with the given numer.
//x - 0, y - 1, z - 2
fn get_dimension(dimension_number:usize)->[usize; 3]{
    let mut dim = [0,0,0];
    dim[dimension_number]=1;
    return dim;
}
