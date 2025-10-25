# Django views.py example - Real-world Django view patterns
from django.shortcuts import render, get_object_or_404, redirect
from django.views import View
from django.views.generic import ListView, DetailView, CreateView, UpdateView, DeleteView
from django.contrib.auth.mixins import LoginRequiredMixin, UserPassesTestMixin
from django.contrib.auth.decorators import login_required
from django.http import JsonResponse, HttpResponse, Http404
from django.urls import reverse_lazy
from django.db.models import Q, Count, Avg
from django.core.paginator import Paginator
from django.contrib import messages
from django.utils.decorators import method_decorator
from django.views.decorators.cache import cache_page
from django.views.decorators.http import require_http_methods

from .models import Product, Category, Review, Order
from .forms import ProductForm, ReviewForm, OrderForm


def home(request):
    """Home page view."""
    featured_products = Product.objects.filter(
        status=Product.STATUS_PUBLISHED,
        featured=True
    )[:6]
    categories = Category.objects.filter(is_active=True)

    context = {
        'featured_products': featured_products,
        'categories': categories,
    }
    return render(request, 'home.html', context)


class ProductListView(ListView):
    """List view for products."""

    model = Product
    template_name = 'products/product_list.html'
    context_object_name = 'products'
    paginate_by = 12

    def get_queryset(self):
        """Filter products based on query parameters."""
        queryset = Product.objects.filter(status=Product.STATUS_PUBLISHED)

        # Filter by category
        category_slug = self.request.GET.get('category')
        if category_slug:
            queryset = queryset.filter(category__slug=category_slug)

        # Search
        query = self.request.GET.get('q')
        if query:
            queryset = queryset.filter(
                Q(name__icontains=query) | Q(description__icontains=query)
            )

        # Sort
        sort = self.request.GET.get('sort', '-created_at')
        valid_sorts = ['name', '-name', 'price', '-price', 'created_at', '-created_at']
        if sort in valid_sorts:
            queryset = queryset.order_by(sort)

        return queryset

    def get_context_data(self, **kwargs):
        """Add additional context."""
        context = super().get_context_data(**kwargs)
        context['categories'] = Category.objects.filter(is_active=True)
        context['current_category'] = self.request.GET.get('category')
        context['search_query'] = self.request.GET.get('q', '')
        return context


class ProductDetailView(DetailView):
    """Detail view for a single product."""

    model = Product
    template_name = 'products/product_detail.html'
    context_object_name = 'product'
    slug_field = 'slug'
    slug_url_kwarg = 'slug'

    def get_queryset(self):
        """Only show published products."""
        return Product.objects.filter(status=Product.STATUS_PUBLISHED)

    def get_context_data(self, **kwargs):
        """Add reviews and related products to context."""
        context = super().get_context_data(**kwargs)
        product = self.object

        # Get reviews
        reviews = product.reviews.select_related('user').all()
        context['reviews'] = reviews
        context['average_rating'] = reviews.aggregate(Avg('rating'))['rating__avg']

        # Related products
        context['related_products'] = Product.objects.filter(
            category=product.category,
            status=Product.STATUS_PUBLISHED
        ).exclude(id=product.id)[:4]

        return context


@method_decorator(cache_page(60 * 15), name='dispatch')
class CategoryListView(ListView):
    """Cached list view for categories."""

    model = Category
    template_name = 'categories/category_list.html'
    context_object_name = 'categories'

    def get_queryset(self):
        """Get active categories with product count."""
        return Category.objects.filter(
            is_active=True
        ).annotate(
            product_count=Count('products')
        ).order_by('name')


class ProductCreateView(LoginRequiredMixin, UserPassesTestMixin, CreateView):
    """Create view for products - staff only."""

    model = Product
    form_class = ProductForm
    template_name = 'products/product_form.html'
    success_url = reverse_lazy('product_list')

    def test_func(self):
        """Check if user is staff."""
        return self.request.user.is_staff

    def form_valid(self, form):
        """Set the created_by field."""
        form.instance.created_by = self.request.user
        messages.success(self.request, 'Product created successfully!')
        return super().form_valid(form)


class ProductUpdateView(LoginRequiredMixin, UserPassesTestMixin, UpdateView):
    """Update view for products."""

    model = Product
    form_class = ProductForm
    template_name = 'products/product_form.html'

    def test_func(self):
        """Check if user can edit this product."""
        product = self.get_object()
        return (
            self.request.user.is_staff or
            self.request.user == product.created_by
        )

    def form_valid(self, form):
        """Show success message."""
        messages.success(self.request, 'Product updated successfully!')
        return super().form_valid(form)


@login_required
@require_http_methods(["POST"])
def add_review(request, product_slug):
    """Add a review to a product."""
    product = get_object_or_404(Product, slug=product_slug)

    # Check if user already reviewed
    if Review.objects.filter(product=product, user=request.user).exists():
        messages.error(request, 'You have already reviewed this product.')
        return redirect('product_detail', slug=product_slug)

    form = ReviewForm(request.POST)
    if form.is_valid():
        review = form.save(commit=False)
        review.product = product
        review.user = request.user
        review.save()
        messages.success(request, 'Review added successfully!')
    else:
        messages.error(request, 'Error adding review.')

    return redirect('product_detail', slug=product_slug)


class ReviewDeleteView(LoginRequiredMixin, UserPassesTestMixin, DeleteView):
    """Delete a review."""

    model = Review
    template_name = 'reviews/review_confirm_delete.html'

    def test_func(self):
        """Check if user owns this review."""
        review = self.get_object()
        return self.request.user == review.user or self.request.user.is_staff

    def get_success_url(self):
        """Redirect to product detail."""
        return reverse_lazy('product_detail', kwargs={'slug': self.object.product.slug})


class OrderListView(LoginRequiredMixin, ListView):
    """List view for user's orders."""

    model = Order
    template_name = 'orders/order_list.html'
    context_object_name = 'orders'
    paginate_by = 10

    def get_queryset(self):
        """Get orders for current user."""
        return Order.objects.filter(user=self.request.user).prefetch_related('items')


class OrderDetailView(LoginRequiredMixin, UserPassesTestMixin, DetailView):
    """Detail view for an order."""

    model = Order
    template_name = 'orders/order_detail.html'
    context_object_name = 'order'

    def test_func(self):
        """Check if user owns this order."""
        order = self.get_object()
        return self.request.user == order.user or self.request.user.is_staff


@login_required
def api_product_search(request):
    """API endpoint for product search."""
    query = request.GET.get('q', '')
    if len(query) < 3:
        return JsonResponse({'error': 'Query must be at least 3 characters'}, status=400)

    products = Product.search(query)[:10]

    results = [
        {
            'id': p.id,
            'name': p.name,
            'slug': p.slug,
            'price': str(p.price),
            'url': p.get_absolute_url(),
        }
        for p in products
    ]

    return JsonResponse({'results': results})


class ProductAPIView(View):
    """API view for products with JSON responses."""

    def get(self, request, *args, **kwargs):
        """Get product list as JSON."""
        products = Product.get_published_products()[:20]

        data = {
            'count': products.count(),
            'products': [
                {
                    'id': p.id,
                    'name': p.name,
                    'price': str(p.price),
                    'category': p.category.name,
                    'in_stock': p.is_in_stock,
                }
                for p in products
            ]
        }
        return JsonResponse(data)


def handler404(request, exception):
    """Custom 404 error handler."""
    return render(request, '404.html', status=404)


def handler500(request):
    """Custom 500 error handler."""
    return render(request, '500.html', status=500)
